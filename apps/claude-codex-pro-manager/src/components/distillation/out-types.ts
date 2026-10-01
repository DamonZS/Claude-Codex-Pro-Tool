/**
 * Output-type catalog adapted from AITracker
 * (modules/distillation/presentation/distill/out-types.ts, used with permission).
 *
 *   capability (→ Skill 库): skill→skill, workflow→brief, prompt→prompt
 *   memory (→ 记忆库):       profile→persona, task→memory
 */
export type OutTypeId = "skill" | "workflow" | "prompt" | "profile" | "task";
export type OutGroupId = "capability" | "memory";
export type CandidateKind = "skill" | "brief" | "prompt" | "persona" | "memory";

export interface OutTypeMeta {
  readonly id: OutTypeId;
  readonly group: OutGroupId;
  readonly kind: CandidateKind;
  readonly label: string;
  readonly hint: string;
  /** Directive injected into the run prompt when this type is selected. */
  readonly instruction: string;
  readonly color: string;
}

export const OUT_GROUPS: readonly { id: OutGroupId; label: string; dest: string }[] = [
  { id: "capability", label: "能力资产（关于「事」）", dest: "Skill 库" },
  { id: "memory", label: "记忆资产（关于「人」）", dest: "记忆库" },
];

export const OUT_TYPES: readonly OutTypeMeta[] = [
  {
    id: "skill",
    group: "capability",
    kind: "skill",
    label: "Skill",
    hint: "完整包：SKILL.md + 脚本 + 参考",
    instruction: "请产出 Skill 完整包：提炼 SKILL.md、配套脚本与参考，面向可安装可复用。",
    color: "var(--dw-chart-1)",
  },
  {
    id: "workflow",
    group: "capability",
    kind: "brief",
    label: "工作流",
    hint: "轻量：1-2-3 步可复用流程",
    instruction: "请产出轻量工作流：把素材中的流程整理为 1-2-3 步，每步注明产出与校验。",
    color: "var(--dw-chart-3)",
  },
  {
    id: "prompt",
    group: "capability",
    kind: "prompt",
    label: "Prompt",
    hint: "最轻：一段可直接复用的提示词",
    instruction: "请产出一段可直接复用的提示词，包含角色、边界与输出格式。",
    color: "var(--dw-chart-4)",
  },
  {
    id: "profile",
    group: "memory",
    kind: "persona",
    label: "画像",
    hint: "你是谁、什么岗位、偏好如何",
    instruction: "请提炼我的岗位角色、沟通偏好与对解释详略的要求（画像）。",
    color: "var(--dw-chart-2)",
  },
  {
    id: "task",
    group: "memory",
    kind: "memory",
    label: "任务记忆",
    hint: "我们定了什么规矩与决策",
    instruction: "请提炼素材中达成的约定、决策与不可逾越的边界（任务记忆）。",
    color: "var(--dw-chart-5)",
  },
];

export const PROMPT_PRESETS: readonly { id: string; label: string; text: string }[] = [
  { id: "concise", label: "精简 120 行", text: "请把素材精简为 120 行以内的结论与可复用要点，去掉铺垫与重复。" },
  { id: "runnable-scripts", label: "附可执行脚本", text: "请把素材中的操作步骤提炼为可直接执行的脚本或命令序列。" },
  { id: "pitfalls", label: "保留踩坑", text: "请重点保留素材中踩过的坑、失败与边界教训。" },
  { id: "sources", label: "附出处链接", text: "请为每条结论附上素材中的出处链接或会话引用。" },
  { id: "chinese", label: "中文输出", text: "请全部用中文输出。" },
];

export function outTypeMeta(id: OutTypeId): OutTypeMeta {
  return OUT_TYPES.find((meta) => meta.id === id) ?? OUT_TYPES[0];
}

export function isMemoryKind(kind: string | undefined): boolean {
  return kind === "persona" || kind === "memory";
}

/** Kind badge metadata keyed by the persisted candidate kind. */
export function kindMeta(kind: string | undefined): { label: string; color: string } {
  const meta = OUT_TYPES.find((item) => item.kind === kind);
  return meta ? { label: meta.label, color: meta.color } : { label: "任务记忆", color: "var(--dw-chart-5)" };
}
