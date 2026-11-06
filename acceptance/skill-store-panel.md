# Skills 真源库面板 — 通过标准

对应规格：[`spec/skill-store-panel.md`](../spec/skill-store-panel.md)。勾选表示有证据；真机点击项由 Lead 在合并前执行。

- [x] **F1 类型检查**：`npm --prefix apps/claude-codex-pro-manager run check` 退出码 0。
- [x] **F2 生产构建**：`npm --prefix apps/claude-codex-pro-manager run vite:build` 退出码 0，`dist/assets` 生成新的 `index-*.js` / `index-*.css`。
- [x] **F3 挂载位置**：`ToolsAndPluginsScreen` 的 `<div className="stack">` 内，`<SkillStorePanel actions={actions} />` 位于 `<UnifiedToolInventoryPanel ... />` 之前。
- [x] **F4 三处登记**：`refreshSkillStore` / `importSkillIntoStore` / `syncStoreSkill` 同时存在于 `createActionsShape()`、`actionsRef.current` 与 `useMemo<AppActions>`，无 TS2741。
- [ ] **F5 挂载即读取**（真机，Lead）：打开工具页触发 `list_skill_store_entries`，面板显示真源库 `root`（只读）与条目；命令失败时保留旧数据并有失败提示。
- [ ] **F6 来源映射**（真机，Lead）：`sourceApp` 为 `codex` / `claude` / `unknown` 时分别显示 Codex / Claude / 手动放入。
- [ ] **F7 分发成功路径**（真机，Lead）：点「分发到 Codex」发出 `sync_store_skill`，参数 `{ request: { id, app: "codex" } }`，成功后列表按载荷刷新；Claude 同理。
- [ ] **F8 空态文案**（真机，Lead）：`entries` 为空时显示「真源库还没有 Skill：在下方列表里点『收进真源』」。
- [ ] **F9 收进真源**（真机，Lead）：`kind === "skill"` 且该应用可用且有 `sourcePath` 的 chip 旁出现「收进真源」；点击发出 `import_skill_into_store`，参数 `{ request: { sourcePath, sourceApp } }`，成功后真源库与工具清单均刷新；不满足条件的 chip 无该按钮。
- [ ] **F10 既有逻辑不变**（代码评审 + 真机，Lead）：chip 的复制/启用开关行为与 `agent-toggle` 样式未改。
- [ ] **F11 失败与禁用**（真机，Lead）：操作中按钮禁用；失败时列表不被清空，仅右下角失败提示。

## 非通过条件

- 面板位于工具清单之后，或任一 action 漏登记导致编译失败。
- 命令参数未包裹 `request`，或把载荷读成 `result.payload.root`。
- 失败时清空列表、抛未捕获异常，或把失败载荷当成功数据渲染。
- 新增 CSS、第三方依赖，或改动 Rust 文件 / 预览 mock。