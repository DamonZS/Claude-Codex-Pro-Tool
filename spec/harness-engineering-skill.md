# Harness Engineering Skill 封装与备份

## 背景

项目已经形成 Harness Engineering 的基础文档、理论文档和项目内工作流。为了让这套方法论可以跨项目复用，需要将其封装成 Codex Skill，并安装到本机 Codex Skill 目录，使未来对话可以通过 Skill 触发同样的工作方式。

同时，用户要求把 Skill 备份一份到桌面，便于迁移、手动检查或复制到其他环境。

## 目标

本次要完成：

- 继续扩展 `docs/harness-engineering-theory.md`，补充 Skill 化后的跨项目使用方式。
- 创建可安装 Skill：`harness-engineering`。
- Skill 必须包含精简的 `SKILL.md`，能指导代理执行先上下文、先规格、先验收、先验证的工作流。
- Skill 必须支持初始化模式：在任意项目中调用该 Skill 并说“初始化”时，代理应生成或补齐该项目的 Harness Engineering 文档基线。
- Skill 必须包含必要参考资料，避免 `SKILL.md` 过长。
- Skill 应提供可执行初始化脚本，用于稳定创建 `AGENTS.md`、`spec/`、`acceptance/` 和基线文档。
- Skill 必须安装到 `C:/Users/Damon/.codex/skills/harness-engineering`。
- Skill 必须备份到桌面。
- Skill 必须通过基础结构验证。

本次不包含：

- 修改应用源码。
- 修改构建、测试、发布或 CI 流程。
- 创建需要联网的安装器。
- 自动推送或发布 Skill。
- 清理仓库中已有的无关源码改动。

## 用户视角描述

用户希望之后在任意 Codex 对话中，可以通过 `$harness-engineering` 或相关触发描述，让代理加载这套工作流。

用户也希望在桌面看到一份备份，必要时可以复制到其他机器或其他 Codex Skill 目录。

## 功能要求

- 新增或更新 `docs/harness-engineering-theory.md`，说明 Skill 化使用方式和维护策略。
- 新增 `spec/harness-engineering-skill.md`。
- 新增 `acceptance/harness-engineering-skill.md`。
- 使用 Skill Creator 规则创建 `harness-engineering` Skill。
- Skill 的 `SKILL.md` 必须包含合法 YAML frontmatter，且 `name` 为 `harness-engineering`。
- Skill 的 `description` 必须以 `Use when` 开头，并描述触发场景，不堆叠完整流程。
- Skill 必须包含 `agents/openai.yaml`。
- Skill 必须包含至少一份参考资料，承载理论细节、模板或验收清单。
- Skill 必须包含 `scripts/init_harness.py` 或等价脚本。
- 初始化脚本必须接受目标项目路径参数。
- 初始化脚本必须能在目标项目中创建：
  - `AGENTS.md`
  - `docs/harness-engineering-theory.md`
  - `spec/harness-engineering-baseline.md`
  - `acceptance/harness-engineering-baseline.md`
- 初始化脚本不得覆盖已有 `AGENTS.md`，除非调用方显式传入覆盖参数。
- Skill 必须安装在 `C:/Users/Damon/.codex/skills/harness-engineering`。
- 桌面必须存在备份目录或压缩包。
- 必须运行 Skill 基础验证脚本或等价检查。

## UI / 交互要求

本次不涉及应用 UI。

## 数据与接口要求

本次不涉及运行时数据库或 API。

## 技术约束

- 只修改 Markdown 文档和 Skill 目录文件。
- 不修改应用源码。
- 不回滚已有无关工作区改动。
- Skill 内容应精简，详细理论放在 `references/` 中。
- 桌面备份不得覆盖不相关已有文件；如目标已存在，应使用时间戳或先删除同名 Skill 备份目录后重新复制。

## 交付范围

- `docs/harness-engineering-theory.md`
- `spec/harness-engineering-skill.md`
- `acceptance/harness-engineering-skill.md`
- `C:/Users/Damon/.codex/skills/harness-engineering/`
- 桌面 Skill 备份
