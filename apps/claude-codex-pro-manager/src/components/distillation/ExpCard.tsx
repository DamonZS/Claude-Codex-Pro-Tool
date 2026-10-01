import { useEffect, useMemo, useState } from "react";
import { ArrowRight, Check, FileCode2, FolderOpen, Pencil, RefreshCw, Rocket, Save, ShieldCheck, Sparkles, X } from "lucide-react";

import type { AppActions } from "@/lib/actions";
import type { DistillationCandidate, DistillationSkillAgent, DistillationWorkbenchSession } from "@/types";

import { candidateRefs, formatDateTime, resolveCandidateSource } from "./common";
import { renderMarkdown } from "./markdown";
import { isMemoryKind, kindMeta } from "./out-types";
import { qualifySkillFiles, type QualifyKind, type SkillQualification } from "./qualify";
import { buildSkillFiles, packageRootName, type PkgFile } from "./skill-files";

function Act({ icon: Icon, label, onClick, disabled }: { icon: typeof Pencil; label: string; onClick?: () => void; disabled?: boolean }) {
  return <button className="dw-act" disabled={disabled} onClick={onClick} type="button"><Icon aria-hidden="true" />{label}</button>;
}

function Modal({ title, children, onClose }: { title: string; children: React.ReactNode; onClose: () => void }) {
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose]);
  return (
    <div className="dw-modal-layer" onClick={onClose} role="presentation">
      <div aria-label={title} aria-modal="true" className="dw-modal" onClick={(event) => event.stopPropagation()} role="dialog">
        <div className="dw-modal-head">
          <h3>{title}</h3>
          <button aria-label="关闭" className="dw-icon-button" onClick={onClose} type="button"><X aria-hidden="true" /></button>
        </div>
        {children}
      </div>
    </div>
  );
}

/** Package file tree + viewer/editor adapted from AITracker PkgBrowser. */
function PkgBrowser({ files, root, color, editing, onChange }: { files: PkgFile[]; root: string; color: string; editing: boolean; onChange: (value: string) => void }) {
  const [local, setLocal] = useState<PkgFile[]>(files);
  const [active, setActive] = useState(files[0]?.path ?? "SKILL.md");
  useEffect(() => setLocal(files), [files]);
  const current = local.find((file) => file.path === active) ?? local[0];
  if (!current) return null;
  const isMarkdown = current.path.endsWith(".md");
  return (
    <div className="dw-pkg" style={{ ["--dw-accent" as string]: color }}>
      <div className="dw-pkg-tree">
        <div className="dw-pkg-root"><FolderOpen aria-hidden="true" />{root}/</div>
        <ul>
          {local.map((file) => (
            <li key={file.path}>
              <button className={file.path === current.path ? "active" : ""} onClick={() => setActive(file.path)} type="button">
                <FileCode2 aria-hidden="true" /><span>{file.path}</span>
              </button>
            </li>
          ))}
        </ul>
      </div>
      <div className="dw-pkg-view">
        <div className="dw-pkg-view-head"><span>{root}/{current.path}</span><span>{current.content.split("\n").length} 行</span></div>
        {editing ? (
          <textarea
            onChange={(event) => {
              const value = event.target.value;
              setLocal((list) => list.map((file) => (file.path === current.path ? { ...file, content: value } : file)));
              if (current.path === files[0]?.path) onChange(value);
            }}
            value={current.content}
          />
        ) : isMarkdown ? (
          <div className="dw-md" dangerouslySetInnerHTML={{ __html: renderMarkdown(current.content) }} />
        ) : (
          <pre>{current.content}</pre>
        )}
      </div>
    </div>
  );
}

/** Save & install dialog: name + live quality check + target agents. */
function SaveModal({
  actions,
  candidate,
  draft,
  skillAgents,
  onClose,
  onSaved,
}: {
  actions: AppActions;
  candidate: DistillationCandidate;
  draft: string;
  skillAgents: readonly DistillationSkillAgent[];
  onClose: () => void;
  onSaved: () => void;
}) {
  const [name, setName] = useState(() => packageRootName(candidate));
  const [targets, setTargets] = useState<string[]>(() => skillAgents.map((agent) => agent.id));
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const kindLabel = kindMeta(candidate.kind).label;
  const files = useMemo(() => buildSkillFiles(draft, candidate), [candidate, draft]);
  const qualification = useMemo<SkillQualification | null>(() => {
    if (candidate.kind !== "skill" && candidate.kind !== "prompt" && candidate.kind !== "brief") return null;
    return files.length ? qualifySkillFiles(files, candidate.kind as QualifyKind) : null;
  }, [candidate.kind, files]);
  const warnCount = qualification?.checks.filter((check) => check.severity === "warn" && !check.pass).length ?? 0;
  const errorCount = qualification?.checks.filter((check) => check.severity === "error" && !check.pass).length ?? 0;
  const allSelected = targets.length === skillAgents.length;

  const save = async () => {
    if (!name.trim() || targets.length === 0) return;
    setSaving(true);
    setError("");
    const result = await actions.saveDistillationOutput({
      candidateId: candidate.id,
      target: candidate.kind ?? "skill",
      skillId: name.trim(),
      files: files.map((file) => ({ path: file.path, content: file.content })),
      agents: targets,
    });
    setSaving(false);
    if (result?.status === "ok") onSaved();
    else setError(result?.message || "保存失败，未写入任何文件。");
  };

  return (
    <Modal onClose={onClose} title={`保存并安装${kindLabel}`}>
      <div className="dw-save">
        <label className="dw-field">
          <span>名称</span>
          <input onChange={(event) => setName(event.target.value)} value={name} />
        </label>
        {qualification ? (
          <div className={`dw-quality${qualification.pass ? " pass" : " fail"}`}>
            <div className="dw-quality-head">
              <ShieldCheck aria-hidden="true" />
              {qualification.pass
                ? warnCount > 0 ? `质检合格 · ${warnCount} 条建议` : "质检合格"
                : `质检不合格（${errorCount} 项）`}
            </div>
            {qualification.checks.some((check) => !check.pass) ? (
              <ul>
                {qualification.checks.filter((check) => !check.pass).map((check) => (
                  <li key={check.id}>
                    <span className={check.severity}>✗</span>
                    <span>{check.label}{check.severity === "warn" ? "（建议）" : ""}</span>
                    {check.detail ? <small>· {check.detail}</small> : null}
                  </li>
                ))}
              </ul>
            ) : null}
          </div>
        ) : null}
        <div className="dw-targets">
          <div className="dw-targets-head">
            <span>安装目标</span>
            <button className="dw-pill-button" onClick={() => setTargets(allSelected ? [] : skillAgents.map((agent) => agent.id))} type="button">{allSelected ? "清空选择" : "全选"}</button>
          </div>
          {skillAgents.length ? (
            <div className="dw-targets-grid">
              {skillAgents.map((agent) => {
                const on = targets.includes(agent.id);
                return (
                  <button
                    aria-pressed={on}
                    className={on ? "on" : ""}
                    key={agent.id}
                    onClick={() => setTargets((current) => (on ? current.filter((item) => item !== agent.id) : [...current, agent.id]))}
                    title={agent.root}
                    type="button"
                  >
                    <span className="dw-target-check">{on ? <Check aria-hidden="true" /> : null}</span>
                    <span className="dw-target-copy"><strong>{agent.label}</strong><small>{agent.root}</small></span>
                  </button>
                );
              })}
            </div>
          ) : <p className="dw-muted">没有发现可安装的 Agent Skill 目录。</p>}
        </div>
        {error ? <p className="dw-error" role="alert">{error}</p> : null}
        <div className="dw-save-actions">
          <span>已选 {targets.length} / {skillAgents.length} 个安装目标</span>
          <button className="dw-pill-button" onClick={onClose} type="button">取消</button>
          <button className="dw-pill-button primary" disabled={saving || !name.trim() || targets.length === 0} onClick={() => void save()} type="button">
            <Save aria-hidden="true" />保存并安装{kindLabel}
          </button>
        </div>
      </div>
    </Modal>
  );
}

/** Persisted result card adapted from AITracker ExpCard (bare variant for the history accordion). */
export function ExpCard({
  actions,
  candidate,
  sessions,
  skillAgents,
  busy,
  onRegenerate,
}: {
  actions: AppActions;
  candidate: DistillationCandidate;
  sessions: readonly DistillationWorkbenchSession[];
  skillAgents: readonly DistillationSkillAgent[];
  busy: boolean;
  onRegenerate: () => void;
}) {
  const memoryAsset = isMemoryKind(candidate.kind);
  const badge = kindMeta(candidate.kind);
  const body = candidate.output || candidate.summary;
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(body);
  const [saveOpen, setSaveOpen] = useState(false);
  useEffect(() => {
    if (!editing) setDraft(body);
  }, [body, editing]);
  const files = useMemo(() => buildSkillFiles(draft, candidate), [draft, candidate]);
  const resolved = useMemo(() => resolveCandidateSource(candidate, sessions), [candidate, sessions]);
  const refs = candidateRefs(candidate);

  return (
    <>
      <article className="dw-exp" style={{ ["--dw-accent" as string]: badge.color }}>
        <div className="dw-exp-meta">
          <span>{formatDateTime(candidate.createdAt)}</span>
          <span>· {candidate.mode === "offline" ? "离线回退（确定性）" : candidate.modelId || candidate.mode || "model"}</span>
          <span>· 素材：{refs.length} 段 · {resolved.sources.join(" / ")}</span>
          {resolved.projectKeys.length ? <span>· 项目：{resolved.projectKeys.join(" / ")}</span> : null}
        </div>
        {memoryAsset ? (
          <>
            <div className="dw-exp-memory">
              <div className="dw-exp-memory-title">
                <Sparkles aria-hidden="true" />
                <strong>{candidate.title}</strong>
                <span>{badge.label}</span>
              </div>
              {editing
                ? <textarea onChange={(event) => setDraft(event.target.value)} rows={8} value={draft} />
                : <p>{draft}</p>}
            </div>
            <div className="dw-exp-actions">
              <Act icon={Pencil} label={editing ? "完成编辑" : "编辑"} onClick={() => setEditing((value) => !value)} />
              <button className="dw-act primary" onClick={() => void actions.openDistillationLibrary(candidate.kind ?? "memory")} type="button"><ArrowRight aria-hidden="true" />去记忆库查看</button>
              <Act disabled={busy} icon={RefreshCw} label="重新生成" onClick={onRegenerate} />
            </div>
          </>
        ) : (
          <>
            <PkgBrowser color={badge.color} editing={editing} files={files} onChange={setDraft} root={packageRootName(candidate)} />
            <div className="dw-exp-actions">
              <Act icon={Pencil} label={editing ? "完成编辑" : "编辑"} onClick={() => setEditing((value) => !value)} />
              <button className="dw-act primary" onClick={() => setSaveOpen(true)} type="button"><Rocket aria-hidden="true" />保存并安装{badge.label}</button>
              <Act disabled={busy} icon={RefreshCw} label="重新生成" onClick={onRegenerate} />
              {candidate.status === "saved" && candidate.savedPath ? <span className="dw-exp-saved">已保存至 {candidate.savedPath}</span> : null}
            </div>
          </>
        )}
      </article>
      {saveOpen ? (
        <SaveModal
          actions={actions}
          candidate={candidate}
          draft={draft}
          onClose={() => setSaveOpen(false)}
          onSaved={() => setSaveOpen(false)}
          skillAgents={skillAgents}
        />
      ) : null}
    </>
  );
}
