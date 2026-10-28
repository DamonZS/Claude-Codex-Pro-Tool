//! Distillation agent: turn a free-form user instruction into an intent
//! (`session` / `skill` / `pitfall`), decide whether the material is worth
//! distilling at all, and run the existing pipeline with that decision applied.

use super::{
    CommandResult, DistillAgentPayload, DistillAgentRequest, DistillationRunPayload,
    DistillationWorkbenchRequest,
};

pub const MAX_INSTRUCTION_CHARS: usize = 2_000;
pub const INTENT_SESSION: &str = "session";
pub const INTENT_SKILL: &str = "skill";
pub const INTENT_PITFALL: &str = "pitfall";

/// Phrases that mean "do not distil this".
const SKIP_MARKERS: [&str; 6] = ["不蒸馏", "别蒸馏", "不用蒸馏", "不需要蒸馏", "跳过", "别记"];
const SKILL_MARKERS: [&str; 5] = ["技能", "skill", "做成技能", "技能包", "打包"];
const PITFALL_MARKERS: [&str; 6] = ["踩坑", "坑", "教训", "失败", "报错", "绕行"];
const WORKBENCH_MARKERS: [&str; 3] = ["蒸馏", "提炼", "总结"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionPlan {
    pub intent: String,
    /// Kind handed to the existing distillation pipeline.
    pub kind: String,
    pub confidence: u8,
    pub skipped: bool,
    pub reason: String,
}

fn contains_any<'a>(haystack: &str, markers: &'a [&'a str]) -> Option<&'a str> {
    markers
        .iter()
        .find(|marker| haystack.contains(*marker))
        .copied()
}

/// Decide what a user instruction asks for. Pure and deterministic so it can be
/// unit-tested without a model call; the model only fills the artifact.
pub fn plan_instruction(instruction: &str) -> InstructionPlan {
    let text = instruction.trim().to_ascii_lowercase();
    if let Some(marker) = contains_any(&text, &SKIP_MARKERS) {
        return InstructionPlan {
            intent: INTENT_SESSION.into(),
            kind: "memory".into(),
            confidence: 95,
            skipped: true,
            reason: format!("指令包含「{marker}」，按不蒸馏处理。"),
        };
    }
    if let Some(marker) = contains_any(&text, &SKILL_MARKERS) {
        return InstructionPlan {
            intent: INTENT_SKILL.into(),
            kind: "skill".into(),
            confidence: 90,
            skipped: false,
            reason: format!("指令命中「{marker}」，按技能蒸馏执行。"),
        };
    }
    if let Some(marker) = contains_any(&text, &PITFALL_MARKERS) {
        return InstructionPlan {
            intent: INTENT_PITFALL.into(),
            kind: "memory".into(),
            confidence: 85,
            skipped: false,
            reason: format!("指令命中「{marker}」，按踩坑蒸馏执行。"),
        };
    }
    let confidence = if contains_any(&text, &WORKBENCH_MARKERS).is_some() {
        70
    } else {
        40
    };
    InstructionPlan {
        intent: INTENT_SESSION.into(),
        kind: "memory".into(),
        confidence,
        skipped: false,
        reason: "未命中具体类型，按会话蒸馏执行。".into(),
    }
}

/// Guard rails with stable error codes, mirroring the existing distillation
/// validation style.
pub fn validate_agent_request(request: &DistillAgentRequest) -> Result<(), (String, String)> {
    if request.instruction.trim().is_empty() {
        return Err((
            "errors.distillation.emptyInstruction".into(),
            "请先输入蒸馏指令（例如「把这会话蒸成技能」）。".into(),
        ));
    }
    if request.instruction.chars().count() > MAX_INSTRUCTION_CHARS {
        return Err((
            "errors.distillation.instructionTooLong".into(),
            format!("蒸馏指令不能超过 {MAX_INSTRUCTION_CHARS} 个字符。"),
        ));
    }
    if request.selections.is_empty() && request.session_ids.is_empty() {
        return Err((
            "errors.distillation.emptySelection".into(),
            "请先选择至少一个会话。".into(),
        ));
    }
    Ok(())
}

/// Intent-specific guidance appended to the caller's own prompt.
pub fn augment_prompt(intent: &str, prompt: &str) -> String {
    let guidance = match intent {
        INTENT_SKILL => {
            "本次要产出可安装的 Skill：必须是可直接执行的能力说明，包含触发条件与操作步骤，禁止空骨架与占位文本。"
        }
        INTENT_PITFALL => {
            "本次只提炼踩坑与教训：每条要写清现象、根因、修复做法与验证方式，不要泛泛而谈。"
        }
        _ => "本次蒸馏会话要点：保留可复用的结论、决策与验证方式。",
    };
    let prompt = prompt.trim();
    if prompt.is_empty() {
        guidance.to_string()
    } else {
        format!("{prompt}\n\n{guidance}")
    }
}

/// Run the distillation agent: validate, plan, then either skip with a visible
/// reason or delegate to the existing workbench pipeline with the plan applied.
pub async fn run_distillation_agent(
    request: DistillAgentRequest,
) -> CommandResult<DistillAgentPayload> {
    let payload =
        |decision: InstructionPlan, run: Option<DistillationRunPayload>| DistillAgentPayload {
            intent: decision.intent,
            kind: decision.kind,
            confidence: decision.confidence,
            skipped: decision.skipped,
            reason: decision.reason,
            run,
        };
    if let Err((code, message)) = validate_agent_request(&request) {
        return super::failed(&message, {
            let mut skipped = payload(plan_instruction(""), None);
            skipped.reason = format!("{message}（{code}）");
            skipped.skipped = true;
            skipped
        });
    }
    let decision = plan_instruction(&request.instruction);
    if decision.skipped {
        let reason = decision.reason.clone();
        return super::ok(&reason, payload(decision, None));
    }
    let workbench = DistillationWorkbenchRequest {
        agent: request.agent.clone(),
        session_ids: request.session_ids.clone(),
        selections: request.selections.clone(),
        provider_id: request.provider_id.clone(),
        model_id: request.model_id.clone(),
        kind: decision.kind.clone(),
        mode: request.mode.clone(),
        prompt: augment_prompt(&decision.intent, &request.prompt),
    };
    let result = super::run_distillation_workbench(workbench).await;
    let status = result.status.clone();
    let message = result.message.clone();
    let run = result.payload;
    let response = payload(decision, Some(run));
    if status == "ok" {
        super::ok(&message, response)
    } else {
        super::failed(&message, response)
    }
}

#[cfg(test)]
mod tests {
    use super::super::DistillationSessionSelection;
    use super::*;

    fn request(instruction: &str, selections: usize) -> DistillAgentRequest {
        DistillAgentRequest {
            instruction: instruction.into(),
            agent: "codex".into(),
            session_ids: Vec::new(),
            selections: (0..selections)
                .map(|index| DistillationSessionSelection {
                    agent: "codex".into(),
                    session_id: format!("session-{index}"),
                    start_index: None,
                    end_index: None,
                })
                .collect(),
            provider_id: "site".into(),
            model_id: "gpt-6.1-sol".into(),
            mode: "online".into(),
            prompt: String::new(),
        }
    }

    #[test]
    fn intent_keywords_pick_the_right_kind() {
        assert_eq!(plan_instruction("把这会话做成技能包").intent, INTENT_SKILL);
        assert_eq!(plan_instruction("帮我蒸成 skill").kind, "skill");
        assert_eq!(plan_instruction("只提炼踩坑与教训").intent, INTENT_PITFALL);
        assert_eq!(plan_instruction("总结一下这个会话").intent, INTENT_SESSION);
        assert_eq!(plan_instruction("随便看看").confidence, 40);
    }

    #[test]
    fn skip_markers_short_circuit_the_run() {
        let plan = plan_instruction("这次不用蒸馏了");
        assert!(plan.skipped);
        assert!(plan.reason.contains("不用蒸馏"));
    }

    #[test]
    fn empty_instruction_and_selection_return_readable_codes() {
        let (code, message) = validate_agent_request(&request("   ", 1)).unwrap_err();
        assert_eq!(code, "errors.distillation.emptyInstruction");
        assert!(message.contains("蒸馏指令"));
        let (code, _) = validate_agent_request(&request("蒸成技能", 0)).unwrap_err();
        assert_eq!(code, "errors.distillation.emptySelection");
        assert!(validate_agent_request(&request("蒸成技能", 2)).is_ok());
        let too_long = "x".repeat(MAX_INSTRUCTION_CHARS + 1);
        let (code, _) = validate_agent_request(&request(&too_long, 1)).unwrap_err();
        assert_eq!(code, "errors.distillation.instructionTooLong");
    }

    #[test]
    fn prompt_augmentation_carries_the_intent_guidance() {
        let skill = augment_prompt(INTENT_SKILL, "用户补充");
        assert!(skill.starts_with("用户补充"));
        assert!(skill.contains("Skill"));
        let pitfall = augment_prompt(INTENT_PITFALL, "");
        assert!(pitfall.contains("踩坑"));
    }
}
