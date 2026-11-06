# Skills 真源库面板

## 背景

CCP 已能把 Claude、Codex 及各 agent 的 Skill 扫描进同一张清单（`scan_unified_tool_inventory`），也支持一次性跨应用复制。但本机没有"统一保存"的 Skill 真源：同一个 Skill 装在多个应用时各存一份，用户看不到本机到底收拢了哪些 Skill，也没有把已有 Skill 分发到另一应用的稳定入口。

后端已经落地真源库（`crates/claude-codex-pro-core/src/skill_store.rs`，根目录 `<CCP home>/skills`）与三条命令：`list_skill_store_entries`、`import_skill_into_store`、`sync_store_skill`。本规格只做前端接线：在工具页暴露真源库列表，并给工具清单里的 Skill chip 增加"收进真源"入口。

## 用户视角

用户打开「Claude、Codex 工具与插件」页，先在顶部看到「Skills 真源库」面板：真源库目录、已收录的 Skill（名称、来源应用、导入时间），每个条目可直接分发到 Codex 或 Claude。下方工具清单里，某个应用已安装的 Skill 旁有「收进真源」按钮，点一下即把该应用目录里的 Skill 复制进真源库；真源库为空时面板给出明确引导文案。

## 功能要求

- F1：在 `ToolsAndPluginsScreen`（工具页）中，`SkillStorePanel` 渲染在既有 `UnifiedToolInventoryPanel` **之前**，标题为「Skills 真源库」。
- F2：面板挂载时调用 `actions.refreshSkillStore()`（命令 `list_skill_store_entries`），把返回的 `root` 与 `entries` 显示出来；`root` 以只读文本呈现，不可编辑。
- F3：条目列表展示 `name`（为空时回退 `id`）、来源应用与导入时间；来源映射为 `codex` → Codex、`claude` → Claude、`unknown` → 手动放入，其它值原样显示。
- F4：每个条目提供「分发到 Codex」「分发到 Claude」两个按钮，分别调用 `actions.syncStoreSkill(id, "codex")` / `actions.syncStoreSkill(id, "claude")`（命令 `sync_store_skill`）；成功后用返回的载荷刷新列表。
- F5：真源库为空时显示提示文案：「真源库还没有 Skill：在下方列表里点『收进真源』」。
- F6：`UnifiedToolInventoryPanel` 中，`asset.kind === "skill"` 且该应用 `state.available && state.sourcePath` 的 chip 旁提供「收进真源」按钮，调用 `actions.importSkillIntoStore(state.sourcePath, app)`（命令 `import_skill_into_store`）；成功后依次刷新真源库与工具清单。既有 chip 的复制/开关点击逻辑与样式保持不变。
- F7：任何一次操作失败（action 返回 `null`）时不得清空已有列表，只在右下角给出失败提示；操作进行中禁用相关按钮，避免重复提交。

## 非目标

- 不修改任何 Rust 文件、`Cargo.toml`、命令签名或返回结构；真源库的落盘、导入与分发规则由后端决定。
- 不做真源库条目的删除、重命名、编辑或排序，不做双向同步与冲突合并。
- 不改动各应用原有 skills 目录结构，不接管启用/禁用状态。
- 不新增 CSS 文件、全局样式或第三方依赖；不修改预览 mock（`tauriPreviewMock.ts`）。
- 不改动 `Spec/` 之外的构建脚本、CI 与发布流程。

## 技术约束

- 仅允许改动：`src/components/SkillStorePanel.tsx`（新建）、`src/screens.tsx`、`src/lib/actions.ts`、`src/App.tsx`、`src/types.ts`。
- 命令统一经 `call<CommandResult<SkillStorePayload>>(命令名, 参数)` 调用；Rust 的 `ok()`/`failed()` 把 payload 字段展开到顶层，故前端直接读取 `root` / `entries`。
- 参数按后端签名包裹：`import_skill_into_store` 传 `{ request: { sourcePath, sourceApp } }`，`sync_store_skill` 传 `{ request: { id, app } }`。
- action 约定：失败时 `notifyResult` 提示后返回 `null`，成功时返回载荷；面板以 `null` 判定失败，不自行拼错误文案。
- 面板只复用现有组件（`Panel`、`Empty`、`InfoRow`、`Button`）与既有类名（`stack`、`action-row`、`context-entry-list`、`context-entry-row`、`unified-tool-row`、`unified-tool-copy`、`info-row`、`spin`）。
- 新增 action 必须在 `createActionsShape()`、`actionsRef.current`、`useMemo<AppActions>` 三处同时登记，否则触发 TS2741。

## 变更记录

- 2026-11-06：新建。对应后端 `skill_store.rs` 与 `settings_commands.rs` 中三条命令的前端接线。