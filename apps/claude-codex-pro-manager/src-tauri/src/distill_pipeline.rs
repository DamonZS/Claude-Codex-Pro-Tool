//! Distillation pipeline, ported from AITracker
//! `src/modules/distillation/application/index.ts` and `api.server.ts`
//! (Copyright (C) 2026 AITracker contributors, used with permission).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use claude_codex_pro_core::settings::{
    RelayProfile, RelayProtocol, SettingsStore, relay_profile_resolved_api_key,
};
use claude_codex_pro_data::aitracker::{DistillationCandidate, DistillationSourceRef};

use super::aitracker_distillation::{
    ControlledRow, MAX_TITLE, SegmentMaterial, SegmentMessage, candidate_text, candidate_title,
    distillation_input, extract_segment_messages, is_opaque_id, mark_distilled_manifest, safe_text,
    summary_line, utf16_prefix,
};
use super::distill_model_client::{
    ModelCallError, ModelCallOptions, ModelProtocol, ModelTarget, call_model,
};
use super::distill_prompts::prompt_for_kind;
use super::distill_qualify::{
    DistilledFile, Qualification, build_files_for_qualification, failure_summary,
    qualify_skill_files,
};
use super::{
    CommandResult, DistillationCandidatesPayload, DistillationFileInput, DistillationOutputRequest,
    DistillationSavePayload, DistillationSessionSelection, DistillationWorkbenchRequest,
};

pub const MAX_SELECTIONS: usize = 100;
pub const MAX_PROMPT_CHARS: usize = 4_000;
pub const MAX_QUALITY_RETRIES: usize = 2;
pub const OFFLINE_FALLBACK_TEXT: &str =
    "Offline deterministic fallback: model execution was not available.";
const MAX_SKILL_FILES: usize = 32;
const MAX_SKILL_PATH_CHARS: usize = 240;
const MAX_SKILL_FILE_CHARS: usize = 64_000;
const MAX_DELETE: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DistillError {
    Cancelled,
    Failed(String),
}

fn now_ms() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .to_string()
}

pub fn normalize_kind(kind: &str) -> &'static str {
    match kind.trim() {
        "skill" => "skill",
        "brief" => "brief",
        "prompt" => "prompt",
        "persona" => "persona",
        _ => "memory",
    }
}

/// AITracker `validRequest` + `isValidSegmentRef`, with visible reasons.
pub fn validate_selections(selections: &[DistillationSessionSelection]) -> Result<(), String> {
    if selections.is_empty() {
        return Err("请先选择至少一个会话。".into());
    }
    if selections.len() > MAX_SELECTIONS {
        return Err(format!("一次最多蒸馏 {MAX_SELECTIONS} 个会话。"));
    }
    let mut keys = BTreeSet::new();
    for selection in selections {
        if !is_opaque_id(&selection.agent) || !is_opaque_id(&selection.session_id) {
            return Err("选中的会话标识无效，请刷新素材列表后重试。".into());
        }
        match (selection.start_index, selection.end_index) {
            (None, None) => {}
            (Some(start), Some(end)) if start <= end => {}
            _ => return Err("选中的会话片段范围无效（需要 0 ≤ 起点 ≤ 终点）。".into()),
        }
        if !keys.insert(format!("{}:{}", selection.agent, selection.session_id)) {
            return Err("同一会话只能选择一个片段区间。".into());
        }
    }
    Ok(())
}

pub fn profile_protocol(profile: &RelayProfile) -> ModelProtocol {
    if claude_codex_pro_core::relay_config::relay_profile_uses_anthropic_messages(profile) {
        ModelProtocol::Anthropic
    } else if profile.protocol == RelayProtocol::Responses {
        ModelProtocol::Responses
    } else {
        ModelProtocol::OpenAi
    }
}

pub fn model_target(profile: &RelayProfile, model: &str) -> ModelTarget {
    ModelTarget {
        protocol: profile_protocol(profile),
        base_url: profile.base_url.trim().to_string(),
        api_key: relay_profile_resolved_api_key(profile),
        model: model.to_string(),
    }
}

/// Port of `runWithQualityFallback`: for skill/brief/prompt, qualify each
/// result and re-run with feedback appended to the system prompt; the final
/// attempt is accepted even when unqualified. Model errors propagate.
pub async fn run_quality_loop<F, Fut>(
    kind: &str,
    system: &str,
    row_count: usize,
    mut call: F,
) -> Result<String, ModelCallError>
where
    F: FnMut(String) -> Fut,
    Fut: Future<Output = Result<String, ModelCallError>>,
{
    if !matches!(kind, "skill" | "brief" | "prompt") {
        return call(system.to_string()).await;
    }
    let mut prompt = system.to_string();
    let hint = format!("{row_count} 场会话蒸馏产物");
    for attempt in 0..=MAX_QUALITY_RETRIES {
        let text = call(prompt.clone()).await?;
        let files = build_files_for_qualification(text.trim(), kind, &hint);
        let qualification = qualify_skill_files(&files, kind);
        if qualification.pass || attempt >= MAX_QUALITY_RETRIES {
            return Ok(text);
        }
        prompt = format!(
            "{prompt}\n\n【质检反馈·第 {} 次】上次输出不合格：{}\n请针对上述问题修正后重新输出，不要解释、不要添加额外说明。",
            attempt + 1,
            failure_summary(&qualification)
        );
    }
    call(prompt).await
}

struct LoadedMaterial {
    rows: Vec<ControlledRow>,
    materials: Vec<SegmentMaterial>,
    refs: Vec<DistillationSourceRef>,
}

fn is_reasoning_role(role: &str) -> bool {
    matches!(
        role.trim().to_ascii_lowercase().as_str(),
        "thinking" | "reasoning"
    )
}

fn load_material(selections: &[DistillationSessionSelection]) -> Result<LoadedMaterial, String> {
    let (_, _, usage) = super::collect_unified_usage_snapshot();
    let mut loaded = LoadedMaterial {
        rows: Vec::new(),
        materials: Vec::new(),
        refs: Vec::new(),
    };
    for selection in selections {
        let Some(detail) = claude_codex_pro_data::aitracker::session_detail(
            &usage,
            &selection.agent,
            &selection.session_id,
        ) else {
            return Err(
                "所选会话已不在当前本地索引中，请刷新素材列表后重试（errors.distillation.sessionNotFound）。"
                    .into(),
            );
        };
        let summary = &detail.summary;
        let row = ControlledRow {
            source: selection.agent.clone(),
            session_id: selection.session_id.clone(),
            title: safe_text(&summary.project, MAX_TITLE),
            project_key: safe_text(&summary.project, 120),
            model: (!summary.model.trim().is_empty()).then(|| safe_text(&summary.model, 120)),
            started_at: summary.started_at.clone(),
            ended_at: summary.ended_at.clone(),
            turns: summary.events,
            edit_turns: 0,
            retry_turns: 0,
            subagent_calls: 0,
            status: if summary.status.trim().is_empty() {
                "completed".into()
            } else {
                summary.status.clone()
            },
        };
        let (start, end) = match (selection.start_index, selection.end_index) {
            (Some(start), Some(end)) => (start, end),
            _ => (0, 0),
        };
        loaded.refs.push(DistillationSourceRef {
            agent: selection.agent.clone(),
            session_id: selection.session_id.clone(),
            project: summary.project.clone(),
            start_index: start,
            end_index: end,
        });
        if let (Some(start), Some(end)) = (selection.start_index, selection.end_index) {
            // A missing transcript drops the segment instead of failing.
            if let Some(transcript) =
                super::distill_transcript::aitracker_session_transcript_for_distill(
                    &selection.agent,
                    &selection.session_id,
                )
            {
                let messages = transcript
                    .messages
                    .iter()
                    .map(|message| SegmentMessage {
                        role: message.role.clone(),
                        text: message.text.clone(),
                    })
                    .collect::<Vec<_>>();
                let window = extract_segment_messages(&messages, start, end)
                    .into_iter()
                    .filter(|message| !is_reasoning_role(&message.role))
                    .collect::<Vec<_>>();
                if !window.is_empty() {
                    loaded.materials.push(SegmentMaterial {
                        source: selection.agent.clone(),
                        session_id: selection.session_id.clone(),
                        title: Some(row.title.clone()),
                        messages: window,
                    });
                }
            }
        }
        loaded.rows.push(row);
    }
    Ok(loaded)
}

fn ensure_active(task_id: &str) -> Result<(), DistillError> {
    if super::distillation_task_cancelled(task_id) {
        Err(DistillError::Cancelled)
    } else {
        Ok(())
    }
}

fn persist_candidate(candidate: &DistillationCandidate) -> Result<(), DistillError> {
    let _guard = super::distillation_candidates_lock();
    let mut candidates = super::read_distillation_candidates();
    candidates.retain(|item| item.id != candidate.id);
    candidates.push(candidate.clone());
    super::write_distillation_candidates(&candidates)
        .map_err(|_| DistillError::Failed("蒸馏候选保存失败（candidate_store_failed）。".into()))
}

pub fn memory_dir(kind: &str) -> PathBuf {
    claude_codex_pro_core::paths::default_app_state_dir()
        .join("aitracker")
        .join("distilled")
        .join(if kind == "persona" {
            "profiles"
        } else {
            "task-memory"
        })
}

/// Write a persona/task-memory candidate into the local memory library.
pub fn write_memory_file_in(
    dir: &Path,
    candidate: &DistillationCandidate,
) -> anyhow::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let safe_id = candidate
        .id
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
        .collect::<String>();
    let file = dir.join(format!("{safe_id}.md"));
    fs::write(&file, &candidate.output)?;
    Ok(file)
}

/// Port of `runDistillationTask` + `DistillationApplication.start/approve`.
pub async fn execute_distillation(
    request: DistillationWorkbenchRequest,
    selections: Vec<DistillationSessionSelection>,
    task_id: String,
) -> Result<DistillationCandidate, DistillError> {
    super::patch_distillation_task(&task_id, "reading-material", 10, None, None);
    let loaded = tauri::async_runtime::spawn_blocking(move || load_material(&selections))
        .await
        .map_err(|_| DistillError::Failed("素材读取任务失败。".into()))?
        .map_err(DistillError::Failed)?;
    ensure_active(&task_id)?;

    super::patch_distillation_task(&task_id, "generating", 30, None, None);
    let kind = normalize_kind(&request.kind);
    let system = prompt_for_kind(kind, utf16_prefix(&request.prompt, MAX_PROMPT_CHARS));
    let input = distillation_input(&loaded.rows, &loaded.materials);
    let offline = request.mode.trim() == "offline" || request.provider_id.trim() == "offline";
    let (text, mode, provider_id, model_id) = if offline {
        (
            OFFLINE_FALLBACK_TEXT.to_string(),
            "offline",
            "offline".to_string(),
            "offline".to_string(),
        )
    } else {
        let settings = SettingsStore::default().load().unwrap_or_default();
        let profile = settings
            .relay_profiles
            .iter()
            .find(|profile| profile.id == request.provider_id.trim())
            .cloned()
            .ok_or_else(|| {
                DistillError::Failed(
                    "蒸馏需配置模型：所选供应商不存在或已删除（errors.distillation.noModelConfigured）。"
                        .into(),
                )
            })?;
        let model = if request.model_id.trim().is_empty() {
            profile.model.trim().to_string()
        } else {
            request.model_id.trim().to_string()
        };
        if model.is_empty() {
            return Err(DistillError::Failed(
                "蒸馏需配置模型：所选供应商未设置模型 ID（errors.distillation.noModelConfigured）。"
                    .into(),
            ));
        }
        let target = model_target(&profile, &model);
        let cancel_id = task_id.clone();
        let cancel = move || super::distillation_task_cancelled(&cancel_id);
        let (target_ref, input_ref, cancel_ref) = (&target, &input, &cancel);
        let result = run_quality_loop(kind, &system, loaded.rows.len(), move |system| async move {
            call_model(
                target_ref,
                &system,
                input_ref,
                ModelCallOptions::default(),
                cancel_ref,
            )
            .await
        })
        .await;
        match result {
            Ok(text) => (text, "model", profile.id.clone(), model),
            Err(error) if error.code == "ai.cancelled" => return Err(DistillError::Cancelled),
            Err(error) => {
                return Err(DistillError::Failed(format!(
                    "{}（errors.distillation.aiFailed）",
                    error.user_message()
                )));
            }
        }
    };
    ensure_active(&task_id)?;

    super::patch_distillation_task(&task_id, "quality-check", 70, None, None);
    let output = candidate_text(Some(&text), loaded.rows.len(), kind);
    let agents = loaded
        .refs
        .iter()
        .map(|item| item.agent.as_str())
        .collect::<BTreeSet<_>>();
    let agent = if agents.len() == 1 {
        agents.first().copied().unwrap_or("multiple")
    } else {
        "multiple"
    };
    let mut candidate = DistillationCandidate {
        id: format!("distill-{task_id}"),
        agent: agent.to_string(),
        session_id: loaded
            .refs
            .iter()
            .map(|item| item.session_id.as_str())
            .collect::<Vec<_>>()
            .join(","),
        summary: summary_line(&output),
        status: "pending".into(),
        created_at: now_ms(),
        kind: kind.into(),
        title: candidate_title(&loaded.rows, kind),
        output,
        mode: mode.into(),
        provider_id,
        model_id,
        task_id: task_id.clone(),
        source_refs: loaded.refs,
        saved_path: String::new(),
    };
    ensure_active(&task_id)?;

    super::patch_distillation_task(
        &task_id,
        "persisting-candidate",
        90,
        None,
        Some(candidate.clone()),
    );
    persist_candidate(&candidate)?;

    super::patch_distillation_task(
        &task_id,
        "syncing-target",
        95,
        None,
        Some(candidate.clone()),
    );
    if matches!(kind, "persona" | "memory") {
        let path = write_memory_file_in(&memory_dir(kind), &candidate)
            .map_err(|error| DistillError::Failed(format!("写入记忆库失败：{error}")))?;
        candidate.saved_path = path.display().to_string();
    }
    candidate.status = "approved".into();
    persist_candidate(&candidate)?;
    Ok(candidate)
}

fn invalid_skill_name(name: &str) -> bool {
    name.is_empty()
        || name == "."
        || name == ".."
        || name.chars().count() > 64
        || name.chars().any(|ch| {
            matches!(ch, '/' | '\\' | '<' | '>' | ':' | '"' | '|' | '?' | '*') || ch.is_control()
        })
}

fn invalid_relative_path(path: &str) -> bool {
    path.is_empty()
        || path.chars().count() > MAX_SKILL_PATH_CHARS
        || path.starts_with('/')
        || path.contains('\\')
        || path.contains(':')
        || path
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
}

/// Validate (or synthesize) the Skill package files for saving.
pub fn validate_skill_files(
    name: &str,
    files: Option<Vec<DistillationFileInput>>,
    output: &str,
) -> Result<Vec<DistilledFile>, String> {
    if invalid_skill_name(name) {
        return Err("Skill 名称无效：不能为空、不超过 64 个字符，且不能包含 / \\ 等路径字符（errors.distillation.invalidName）。".into());
    }
    let supplied = files.unwrap_or_default();
    let files = if supplied.is_empty() {
        let summary = output.trim();
        let description = utf16_prefix(summary.split('\n').next().unwrap_or(""), 120);
        let mut lines = vec!["---".to_string(), format!("name: {name}")];
        if !description.is_empty() {
            lines.push(format!("description: {description}"));
        }
        lines.extend([
            "---".to_string(),
            String::new(),
            format!("# {name}"),
            String::new(),
            if summary.is_empty() {
                format!("Distilled knowledge note ({name}).")
            } else {
                summary.to_string()
            },
            String::new(),
        ]);
        vec![DistillationFileInput {
            path: "SKILL.md".into(),
            content: lines.join("\n"),
        }]
    } else {
        supplied
    };
    if files.len() > MAX_SKILL_FILES {
        return Err(format!("Skill 文件数量不能超过 {MAX_SKILL_FILES} 个。"));
    }
    let mut seen = BTreeSet::new();
    let mut validated = Vec::new();
    for file in files {
        let path = file.path.trim().to_string();
        if invalid_relative_path(&path) || !seen.insert(path.clone()) {
            return Err(format!(
                "Skill 文件路径无效或重复：{}（须为不含 .. 与反斜杠的相对路径，≤ {MAX_SKILL_PATH_CHARS} 字符）。",
                utf16_prefix(&path, 80)
            ));
        }
        if file.content.chars().count() > MAX_SKILL_FILE_CHARS {
            return Err(format!(
                "Skill 文件 {path} 超过 {MAX_SKILL_FILE_CHARS} 字符上限。"
            ));
        }
        let content = if path == "SKILL.md" {
            mark_distilled_manifest(&file.content)
        } else {
            file.content
        };
        validated.push(DistilledFile { path, content });
    }
    if !seen.contains("SKILL.md") {
        return Err("Skill 文件中必须包含 SKILL.md。".into());
    }
    Ok(validated)
}

#[derive(Debug, Default)]
pub struct InstallOutcome {
    pub written: Vec<PathBuf>,
    pub skipped: Vec<String>,
    pub qualification: Qualification,
}

/// Write a validated Skill package to `<agent root>/<name>/` for each agent.
/// Existing directories are never overwritten.
pub fn install_skill_files(
    name: &str,
    files: &[DistilledFile],
    agents: &[String],
    roots: &BTreeMap<String, Vec<PathBuf>>,
    kind: &str,
) -> Result<InstallOutcome, String> {
    let mut unique = Vec::new();
    for agent in agents {
        let agent = agent.trim();
        if !unique.contains(&agent) {
            unique.push(agent);
        }
    }
    if unique.is_empty() {
        return Err("请至少选择一个目标 Agent。".into());
    }
    let mut targets = Vec::new();
    for agent in unique {
        let Some(root) = roots.get(agent).and_then(|items| items.first()) else {
            return Err(format!(
                "不支持的目标 Agent：{agent}（errors.distillation.invalidAgent）。"
            ));
        };
        targets.push((agent.to_string(), root.join(name)));
    }
    let mut outcome = InstallOutcome::default();
    for (agent, dir) in targets {
        if fs::symlink_metadata(&dir).is_ok() {
            outcome.skipped.push(agent);
            continue;
        }
        let result = (|| -> std::io::Result<()> {
            fs::create_dir_all(&dir)?;
            for file in files {
                let target = file
                    .path
                    .split('/')
                    .fold(dir.clone(), |path, segment| path.join(segment));
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&target, &file.content)?;
            }
            Ok(())
        })();
        if let Err(error) = result {
            return Err(format!("写入 Skill 失败（{agent}）：{error}"));
        }
        outcome.written.push(dir);
    }
    if outcome.written.is_empty() {
        return Err(format!(
            "目标 Agent 中已存在同名 Skill 目录「{name}」，未覆盖任何文件（errors.distillation.skillExists）。"
        ));
    }
    outcome.qualification = qualify_skill_files(files, kind);
    Ok(outcome)
}

fn save_payload(
    candidates: Vec<DistillationCandidate>,
    quality: Qualification,
    written: Vec<String>,
) -> DistillationSavePayload {
    DistillationSavePayload {
        candidates,
        quality,
        written,
    }
}

/// `save_distillation_output`: install a capability candidate as a Skill, or
/// (re)write a memory candidate into the memory library.
pub fn save_output(request: DistillationOutputRequest) -> CommandResult<DistillationSavePayload> {
    let _guard = super::distillation_candidates_lock();
    let mut candidates = super::read_distillation_candidates();
    let fail = |message: &str, candidates: Vec<DistillationCandidate>| {
        super::failed(
            message,
            save_payload(candidates, Qualification::default(), Vec::new()),
        )
    };
    let Some(index) = candidates
        .iter()
        .position(|item| item.id == request.candidate_id.trim())
    else {
        return fail("未找到蒸馏候选。", candidates);
    };
    if !matches!(candidates[index].status.as_str(), "approved" | "saved") {
        return fail("请先审批该蒸馏候选。", candidates);
    }
    let memory_kind = matches!(candidates[index].kind.as_str(), "persona" | "memory");
    let target = request.target.trim();
    if !target.is_empty() && matches!(target, "persona" | "memory") != memory_kind {
        return fail("蒸馏候选类型与目标库类型不匹配。", candidates);
    }
    if memory_kind {
        let kind = candidates[index].kind.clone();
        return match write_memory_file_in(&memory_dir(&kind), &candidates[index]) {
            Ok(path) => {
                let path = path.display().to_string();
                candidates[index].saved_path = path.clone();
                if super::write_distillation_candidates(&candidates).is_err() {
                    return fail("蒸馏候选保存失败。", candidates);
                }
                super::ok(
                    &format!("已写入记忆库：{path}"),
                    save_payload(
                        candidates,
                        Qualification {
                            pass: true,
                            checks: Vec::new(),
                        },
                        vec![path],
                    ),
                )
            }
            Err(error) => fail(&format!("写入记忆库失败：{error}"), candidates),
        };
    }
    let name = request.skill_id.trim().to_string();
    let files = match validate_skill_files(&name, request.files, &candidates[index].output) {
        Ok(files) => files,
        Err(message) => return fail(&message, candidates),
    };
    let roots = super::unified_inventory_roots().agent_skill_roots;
    let agents = request.agents.unwrap_or_default();
    let outcome = match install_skill_files(&name, &files, &agents, &roots, &candidates[index].kind)
    {
        Ok(outcome) => outcome,
        Err(message) => return fail(&message, candidates),
    };
    let written = outcome
        .written
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    candidates[index].status = "saved".into();
    candidates[index].saved_path = written.first().cloned().unwrap_or_default();
    if super::write_distillation_candidates(&candidates).is_err() {
        return fail("Skill 已写入，但蒸馏候选状态保存失败。", candidates);
    }
    let mut message = format!("Skill 已安装到 {} 个 Agent。", written.len());
    if !outcome.skipped.is_empty() {
        message.push_str(&format!(
            "以下 Agent 已存在同名目录，未覆盖：{}。",
            outcome.skipped.join("、")
        ));
    }
    super::ok(
        &message,
        save_payload(candidates, outcome.qualification, written),
    )
}

/// `delete_distillation_candidates`: remove 1..=100 candidates by id.
pub fn delete_candidates(ids: Vec<String>) -> CommandResult<DistillationCandidatesPayload> {
    if ids.is_empty() || ids.len() > MAX_DELETE {
        return super::failed(
            &format!("一次可删除 1–{MAX_DELETE} 个蒸馏候选。"),
            DistillationCandidatesPayload {
                candidates: super::all_distillation_candidates(),
            },
        );
    }
    let ids = ids
        .iter()
        .map(|id| id.trim().to_string())
        .collect::<BTreeSet<_>>();
    let _guard = super::distillation_candidates_lock();
    let mut candidates = super::read_distillation_candidates();
    let before = candidates.len();
    candidates.retain(|item| !ids.contains(&item.id));
    let mut removed = before - candidates.len();
    {
        let mut pending = super::pending_distillation_candidates()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let pending_before = pending.len();
        pending.retain(|item| !ids.contains(&item.id));
        removed += pending_before - pending.len();
    }
    if super::write_distillation_candidates(&candidates).is_err() {
        return super::failed(
            "蒸馏候选保存失败。",
            DistillationCandidatesPayload { candidates },
        );
    }
    drop(_guard);
    super::ok(
        &format!("已删除 {removed} 个蒸馏候选。"),
        DistillationCandidatesPayload {
            candidates: super::all_distillation_candidates(),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::super::distill_model_client::test_support::spawn_mock;
    use super::*;
    use std::time::Duration;

    fn selection(
        agent: &str,
        id: &str,
        range: Option<(usize, usize)>,
    ) -> DistillationSessionSelection {
        DistillationSessionSelection {
            agent: agent.into(),
            session_id: id.into(),
            start_index: range.map(|(start, _)| start),
            end_index: range.map(|(_, end)| end),
        }
    }

    #[test]
    fn distill_validation_rules() {
        assert!(validate_selections(&[]).is_err());
        assert!(validate_selections(&[selection("codex", "s1", None)]).is_ok());
        assert!(validate_selections(&[selection("codex", "s1", Some((2, 2)))]).is_ok());
        assert!(validate_selections(&[selection("codex", "s1", Some((3, 2)))]).is_err());
        assert!(
            validate_selections(&[DistillationSessionSelection {
                start_index: Some(1),
                ..selection("codex", "s1", None)
            }])
            .is_err()
        );
        assert!(validate_selections(&[selection("codex", "../etc", None)]).is_err());
        assert!(validate_selections(&[selection("", "s1", None)]).is_err());
        assert!(
            validate_selections(&[
                selection("codex", "s1", Some((0, 1))),
                selection("codex", "s1", Some((2, 3)))
            ])
            .is_err()
        );
        let many = (0..101)
            .map(|index| selection("codex", &format!("s{index}"), None))
            .collect::<Vec<_>>();
        assert!(validate_selections(&many).is_err());
        assert_eq!(normalize_kind("weird"), "memory");
    }

    #[test]
    fn distill_quality_loop_appends_feedback_to_system_prompt() {
        let unqualified = r#"{"choices":[{"message":{"content":"no skill folder here"}}]}"#;
        let skill = "<folder name=\"Weekly\"><file path=\"SKILL.md\">---\nname: Weekly\ndescription: 为团队提供中文周报写作指导与结构建议，覆盖 weekly report outline 与语气调整\n---\n# Weekly\n</file></folder>";
        let qualified = serde_json::json!({"choices":[{"message":{"content": skill}}]}).to_string();
        let (base, captured, server) =
            spawn_mock(vec![(200, unqualified.into()), (200, qualified)]);
        let target = ModelTarget {
            protocol: ModelProtocol::OpenAi,
            base_url: base,
            api_key: "k".into(),
            model: "m".into(),
        };
        let options = ModelCallOptions {
            timeout: Duration::from_secs(10),
            retry_delay: Duration::from_millis(5),
        };
        let cancel = || false;
        let (target_ref, cancel_ref) = (&target, &cancel);
        let output = tauri::async_runtime::block_on(run_quality_loop(
            "skill",
            "BASE SYSTEM",
            1,
            move |system| async move {
                call_model(target_ref, &system, "input", options, cancel_ref).await
            },
        ))
        .unwrap();
        server.join().unwrap();
        assert!(output.contains("name: Weekly"));
        let requests = captured.try_iter().collect::<Vec<_>>();
        assert_eq!(requests.len(), 2);
        let first: serde_json::Value = serde_json::from_str(&requests[0].body).unwrap();
        let second: serde_json::Value = serde_json::from_str(&requests[1].body).unwrap();
        assert_eq!(first["messages"][0]["content"], "BASE SYSTEM");
        let retry_system = second["messages"][0]["content"].as_str().unwrap();
        assert!(retry_system.starts_with("BASE SYSTEM\n\n【质检反馈·第 1 次】上次输出不合格："));
        assert!(retry_system.contains(
            "SKILL.md 含 name / description frontmatter（frontmatter 缺少 name 或 description）"
        ));
        assert!(
            retry_system.ends_with("\n请针对上述问题修正后重新输出，不要解释、不要添加额外说明。")
        );
        assert_eq!(second["messages"][1]["content"], "input");
    }

    #[test]
    fn distill_quality_loop_accepts_final_attempt_and_skips_memory() {
        let mut calls = 0;
        let output = tauri::async_runtime::block_on(run_quality_loop("brief", "S", 1, |_| {
            calls += 1;
            async { Ok(String::new()) }
        }))
        .unwrap();
        assert_eq!(output, "");
        assert_eq!(calls, 3, "first attempt + 2 corrections");

        let mut calls = 0;
        tauri::async_runtime::block_on(run_quality_loop("memory", "S", 1, |_| {
            calls += 1;
            async { Ok("x".to_string()) }
        }))
        .unwrap();
        assert_eq!(calls, 1);

        let error = tauri::async_runtime::block_on(run_quality_loop("skill", "S", 1, |_| async {
            Err(ModelCallError {
                code: "ai.provider-unavailable",
                detail: None,
            })
        }))
        .unwrap_err();
        assert_eq!(error.code, "ai.provider-unavailable");
    }

    #[test]
    fn distill_save_writes_marked_skill_and_refuses_existing_dir() {
        let temp = tempfile::tempdir().unwrap();
        let codex_root = temp.path().join("codex-skills");
        let claude_root = temp.path().join("claude-skills");
        let roots = BTreeMap::from([
            ("codex".to_string(), vec![codex_root.clone()]),
            ("claude-code".to_string(), vec![claude_root.clone()]),
        ]);
        let files = validate_skill_files("weekly", None, "周报助手\n正文").unwrap();
        assert!(files[0].content.starts_with(
            "---\naitracker-origin: distilled\nname: weekly\ndescription: 周报助手\n---\n\n# weekly\n\n周报助手\n正文\n"
        ));
        let outcome = install_skill_files(
            "weekly",
            &files,
            &["codex".into(), "claude-code".into()],
            &roots,
            "skill",
        )
        .unwrap();
        assert_eq!(outcome.written.len(), 2);
        let written = fs::read_to_string(codex_root.join("weekly").join("SKILL.md")).unwrap();
        assert!(written.contains("aitracker-origin: distilled"));

        // Existing directory: nothing written, skillExists error.
        fs::write(claude_root.join("weekly").join("SKILL.md"), "original").unwrap();
        let error = install_skill_files("weekly", &files, &["claude-code".into()], &roots, "skill")
            .unwrap_err();
        assert!(error.contains("skillExists"));
        assert_eq!(
            fs::read_to_string(claude_root.join("weekly").join("SKILL.md")).unwrap(),
            "original"
        );
        assert!(install_skill_files("x", &files, &["unknown".into()], &roots, "skill").is_err());
        assert!(install_skill_files("x", &files, &[], &roots, "skill").is_err());
    }

    #[test]
    fn distill_save_validates_names_and_paths() {
        assert!(validate_skill_files("", None, "x").is_err());
        assert!(validate_skill_files("a/b", None, "x").is_err());
        assert!(validate_skill_files("a\\b", None, "x").is_err());
        assert!(validate_skill_files(&"n".repeat(65), None, "x").is_err());
        let file = |path: &str| DistillationFileInput {
            path: path.into(),
            content: "body".into(),
        };
        assert!(validate_skill_files("ok", Some(vec![file("README.md")]), "x").is_err());
        assert!(
            validate_skill_files("ok", Some(vec![file("SKILL.md"), file("../x")]), "x").is_err()
        );
        assert!(
            validate_skill_files("ok", Some(vec![file("SKILL.md"), file("a\\b")]), "x").is_err()
        );
        assert!(
            validate_skill_files("ok", Some(vec![file("SKILL.md"), file("/abs")]), "x").is_err()
        );
        assert!(
            validate_skill_files("ok", Some(vec![file("SKILL.md"), file("SKILL.md")]), "x")
                .is_err()
        );
        let many = (0..33)
            .map(|index| file(&format!("f{index}.md")))
            .collect::<Vec<_>>();
        assert!(validate_skill_files("ok", Some(many), "x").is_err());
        let files = validate_skill_files(
            "ok",
            Some(vec![file("SKILL.md"), file("scripts/run.py")]),
            "x",
        )
        .unwrap();
        assert_eq!(
            files[0].content,
            "---\naitracker-origin: distilled\n---\n\nbody"
        );
        assert_eq!(files[1].path, "scripts/run.py");
    }

    #[test]
    fn distill_memory_file_written_to_library_dir() {
        let temp = tempfile::tempdir().unwrap();
        let candidate = DistillationCandidate {
            id: "distill-task/1".into(),
            agent: "codex".into(),
            session_id: "s".into(),
            summary: "s".into(),
            status: "approved".into(),
            created_at: "1".into(),
            kind: "memory".into(),
            title: "t".into(),
            output: "## 当前目标".into(),
            mode: "model".into(),
            provider_id: "p".into(),
            model_id: "m".into(),
            task_id: "t".into(),
            source_refs: Vec::new(),
            saved_path: String::new(),
        };
        let path = write_memory_file_in(temp.path(), &candidate).unwrap();
        assert_eq!(path.file_name().unwrap(), "distill-task1.md");
        assert_eq!(fs::read_to_string(path).unwrap(), "## 当前目标");
    }
}
