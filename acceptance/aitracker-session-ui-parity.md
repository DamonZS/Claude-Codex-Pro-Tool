# AITracker 会话管理界面还原验收

对应规格：`spec/aitracker-session-ui-parity.md`

## 通过标准

1. `AitrackerSessionPanel` 包含三项统计卡、同排搜索/范围/刷新、Agent 工具标签、日期分组、会话卡片和分页。
2. 每条会话卡显示真实 Agent、项目、Provider、模型、事件/工具调用数、Token 数和状态；列表不使用演示数据。
3. 点击 Codex 等工具只显示该 Agent 的会话，搜索、范围和分页也在完整快照上即时生效，以上操作不调用后端采集；工具图标不全为机器人。
4. 点击会话后列表页被独立历史视图替换；左侧会话可搜索、按日期分组且可切换，右侧显示当前会话的真实记录和蒸馏入口，返回按钮恢复列表。
   本地读取器提供标题时，右侧显示真实标题；当前选中会话始终出现在左侧历史列表中。
5. 会话路由默认不渲染 `历史会话修复`、`Codex 会话管理`、`Claude 会话管理` 等旧 CCP 固定面板；详情不在列表下方追加。
6. 会话页使用 CCP 透明液态玻璃材质，外层导航和窗口结构保持原状；仅有用量事件的 Agent 不将事件冒充为对话正文。
   打开详情时优先显示已采集的事件，对话正文异步加载；用户与 Agent 消息标签明显不同，长消息滚动时标签可见。
7. `npm --prefix apps/claude-codex-pro-manager run check` 通过。
8. `npm --prefix apps/claude-codex-pro-manager run vite:build` 通过。
9. `cargo build --release -p claude-codex-pro-manager -j 2` 通过，并生成默认 `target/release/claude-codex-pro.exe`。
10. `git diff --check` 通过。

## 证据

- 源码检查：`screens.tsx` 的 `AitrackerSessionPanel` 与 `workspace.css` 的会话页选择器。
- 页面源码检查：`SessionManagementScreen` 仅挂载 `AitrackerSessionPanel`，且不包含上一版 CCP 固定面板标题或布局类名。
- 类型检查、工作流测试、Vitest、Vite 构建和 Release 构建输出。

## 非目标

- 不迁入 AITracker 的 Electron/Node 运行时。
- 不改变 Rust 会话采集器或外层导航。
