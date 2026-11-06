# 跨应用复制 Skill

## 背景

cc-switch 的 Skills 管理把 `~/.cc-switch/skills/` 当作唯一源，再按需分发到各应用目录。CCP 目前已经能把 Claude、Codex 及其它 agent 的 Skill 扫描进同一张清单（`crates/claude-codex-pro-core/src/unified_tool_inventory.rs`），但某个 Skill 只装在 A 应用时，用户在 B 应用里只能看到"不可用"，没有把已有 Skill 搬过去的入口。

本规格只做**一次性复制**这一件事，不引入 SSOT 目录，也不接管各应用原有目录。

## 功能要求

- F1：在「Claude、Codex 工具与插件」面板的 Skill 页，某个应用尚未安装某个 Skill 时，该应用图标可点击，点击后把该 Skill 的本地目录复制到目标应用的 skills 根目录。
- F2：复制来源取该 Skill 任一可用来源的本地目录（`source_path`，其下必须存在 `SKILL.md`）；不修改、不删除源目录。
- F3：目标应用已存在同名 Skill 目录时拒绝复制，并提示具体路径。
- F4：能力上限定为 `codex`（`~/.codex/skills`）与 `claude`（`~/.claude/skills`）两个目标；其它应用返回"暂不支持复制到应用：<app>"。
- F5：复制成功后重新扫描清单并回传，使面板即时反映目标应用已可用。

## 非目标

- 不建立 `~/.claude-codex-pro/skills` 之类的 SSOT 目录，不做双向同步或删除（cc-switch 的完整 SSOT 方案不在本次范围）。
- 不复制 MCP 配置与插件。
- 不改变源应用的启用状态，不在目标应用写 `.disabled` 快照。

## 技术约束

- 复用既有 `scan_unified_tool_inventory` 定位资产，不新增清单结构。
- 目录复制只处理普通文件与目录，跳过符号链接。
- 复制失败时不留空目标目录之外的半成品语义：目标目录已创建但内容不完整时，错误信息必须包含源路径与目标路径。

## 变更记录

- 2026-11-06：新建。来源 cc-switch `src-tauri/src/services/skill.rs`（SSOT + 按需同步）的最小切片。