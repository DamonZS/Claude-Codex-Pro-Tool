/**
 * Skill package builder adapted from AITracker ExpCard `buildSkillFiles`
 * (used with permission): the main knowledge file carries the real model
 * output; auxiliary references/scripts/metadata come from fixed templates.
 */
import type { DistillationCandidate } from "@/types";

import { agentLabel, candidateRefs } from "./common";
import { parseFileTags } from "./qualify";

export interface PkgFile {
  readonly path: string;
  readonly content: string;
}

export function suggestSkillName(title: string): string {
  const slug = title
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 40);
  return slug || "distilled-skill";
}

export function packageRootName(candidate: DistillationCandidate): string {
  const base = suggestSkillName(candidate.title || "");
  if (candidate.kind === "prompt") return `${base}-prompt-pack`;
  if (candidate.kind === "brief") return `${base}-workflow-pack`;
  return base;
}

function sourceNames(candidate: DistillationCandidate) {
  return [...new Set(candidateRefs(candidate).map((ref) => agentLabel(ref.agent)))].join(" / ").trim();
}

function isoDate(value: string) {
  const date = new Date(/^\d+$/.test(value) ? Number(value) : value);
  return Number.isNaN(date.getTime()) ? new Date() : date;
}

export function buildSkillFiles(summary: string, candidate: DistillationCandidate): PkgFile[] {
  const baseName = packageRootName(candidate);
  if (candidate.kind === "skill") {
    const parsed = parseFileTags(summary);
    if (parsed.some((file) => file.path === "SKILL.md")) return parsed;
  }
  const slug = suggestSkillName(candidate.title || "");
  const src = sourceNames(candidate) || "近期素材";
  const sessions = candidateRefs(candidate).length;
  const created = isoDate(candidate.createdAt);
  const date = created.toISOString().slice(0, 10);

  if (candidate.kind === "prompt") {
    return [
      {
        path: "SKILL.md",
        content: `---\nname: ${baseName}\ndescription: 由 ${src} 蒸馏得到的可复用 Prompt 能力包\nform: prompt-package\ncreated: ${date}\n---\n\n# ${baseName}\n\n> **类型:** Prompt 能力包\n> **来源:** ${src}\n> **使用方式:** 优先阅读 \`PROMPT.md\`，把其中模板复制到 Agent 系统提示词或工作流节点中。\n\n## 文件说明\n- \`PROMPT.md\`：主提示词模板\n- \`references/usage.md\`：适用方式与接入建议\n`,
      },
      {
        path: "PROMPT.md",
        content: `---\nname: ${slug}\ndescription: 由 ${src} 蒸馏得到的可复用提示词\nform: prompt\ncreated: ${date}\n---\n\n${summary}\n\n## 使用方式\n直接粘贴到Agent的系统提示词或 CLAUDE.md 顶部。\n`,
      },
      {
        path: "references/usage.md",
        content: "# 使用建议\n\n1. 先替换变量占位符，再交给 Agent 执行\n2. 如果任务跨度大，建议把 Prompt 拆到多个步骤节点\n3. 若输出不稳定，补充项目上下文与成功标准\n",
      },
    ];
  }

  if (candidate.kind === "brief") {
    return [
      {
        path: "SKILL.md",
        content: `---\nname: ${baseName}\ndescription: 由 ${src} 蒸馏得到的可复用工作流能力包\nform: workflow-package\ncreated: ${date}\n---\n\n# ${baseName}\n\n> **类型:** 工作流能力包\n> **来源:** ${src}\n> **使用方式:** 先阅读 \`WORKFLOW.md\`，按步骤接入到你的 Agent / 自动化编排里。\n\n## 文件说明\n- \`WORKFLOW.md\`：主工作流文档\n- \`references/pitfalls.md\`：已知风险与修复建议\n`,
      },
      {
        path: "WORKFLOW.md",
        content: `---\nname: ${slug}\ndescription: 由 ${src} 蒸馏得到的可复用工作流\nform: workflow\ncreated: ${date}\n---\n\n${summary}\n`,
      },
      {
        path: "references/pitfalls.md",
        content: "# 历史踩坑与修复\n\n1. SSR 阶段访问 window：改为 useEffect 内读取\n2. 长列表卡顿：虚拟滚动 + memo\n3. 时区偏差：统一 UTC 存储、本地化展示\n",
      },
    ];
  }

  return [
    {
      path: "SKILL.md",
      content: `---\nname: ${slug}\ndescription: 由 ${sessions} 场会话（${src}）蒸馏得到的个人知识 Skill\nlicense: internal\ncreated: ${date}\nallowed-tools: Read, Grep, Glob, Bash(python3 scripts/*)\n---\n\n${summary}\n\n## 参考资料\n- \`references/conventions.md\` 团队与个人编码约定\n- \`references/stack.md\` 技术栈与依赖清单\n- \`references/pitfalls.md\` 历史踩坑与修复方式\n\n## 可执行脚本\n- \`scripts/apply_conventions.py\` 按约定检查/修正当前仓库\n- \`scripts/collect_context.sh\` 采集项目上下文摘要\n`,
    },
    {
      path: "references/conventions.md",
      content: `# 编码约定（蒸馏自 ${sessions} 场会话）\n\n- 组件保持单一职责，超过 200 行拆分\n- 状态优先 URL / query 缓存，避免多份真源\n- 错误处理统一走 toast + 日志，不静默吞异常\n- 命名：函数动词开头，布尔以 is/has 前缀\n`,
    },
    {
      path: "references/stack.md",
      content: "# 技术栈画像\n\n| 层 | 选型 | 备注 |\n| --- | --- | --- |\n| 前端 | React 19 + TypeScript | 严格模式 |\n| 路由 | TanStack Router | 文件式路由 |\n| 样式 | Tailwind CSS v4 | 语义 token |\n| 数据 | TanStack Query | 缓存与失效 |\n",
    },
    {
      path: "references/pitfalls.md",
      content: "# 常见问题与修复\n\n1. SSR 阶段访问 window：改为 useEffect 内读取\n2. 长列表卡顿：虚拟滚动 + memo\n3. 时区偏差：统一 UTC 存储、本地化展示\n",
    },
    {
      path: "scripts/apply_conventions.py",
      content: "#!/usr/bin/env python3\n\"\"\"按蒸馏出的约定检查当前仓库。\"\"\"\nimport pathlib, re, sys\n\nBAD = re.compile(r\"console\\.log\\(\")\n\ndef main() -> int:\n    hits = []\n    for f in pathlib.Path(\"src\").rglob(\"*.ts*\"):\n        if BAD.search(f.read_text(encoding=\"utf-8\", errors=\"ignore\")):\n            hits.append(str(f))\n    for h in hits:\n        print(\"debug log:\", h)\n    return 1 if hits else 0\n\nif __name__ == \"__main__\":\n    sys.exit(main())\n",
    },
    {
      path: "scripts/collect_context.sh",
      content: "#!/usr/bin/env bash\nset -euo pipefail\n# 采集项目上下文，供 Skill 在会话开始时读取\necho \"## 依赖\"; cat package.json | head -40\necho \"## 目录\"; ls -1 src\n",
    },
    {
      path: "assets/metadata.json",
      content: JSON.stringify({ slug, origin: "local-distill", scope: src, sessions, createdAt: created.toISOString(), entry: "SKILL.md" }, null, 2),
    },
  ];
}
