# AITracker 会话管理界面还原验收

对应规格：`spec/aitracker-session-ui-parity.md`

## 通过标准

1. `AitrackerSessionPanel` 包含三项统计卡、同排搜索/范围/刷新、Agent 工具标签、日期分组、会话卡片和分页。
2. 每条会话卡显示真实 Agent、项目、Provider、模型、事件/工具调用数、Token 数和状态；列表不使用演示数据。
3. 会话详情、工具调用分析和蒸馏候选区域保持可用。
4. 会话路由默认不渲染 `历史会话修复`、`Codex 会话管理`、`Claude 会话管理`、独立 Agent 详情和独立会话详情等旧 CCP 固定面板；点击会话后的详情属于 AITRACKER 页面内联详情。
5. 会话页使用 CCP 透明液态玻璃材质，外层导航、窗口结构和数据命令保持原状。
6. `npm --prefix apps/claude-codex-pro-manager run check` 通过。
7. `npm --prefix apps/claude-codex-pro-manager run vite:build` 通过。
8. `cargo build --release -p claude-codex-pro-manager -j 2` 通过，并生成默认 `target/release/claude-codex-pro.exe`。
9. `git diff --check` 通过。

## 证据

- 源码检查：`screens.tsx` 的 `AitrackerSessionPanel` 与 `workspace.css` 的会话页选择器。
- 页面源码检查：`SessionManagementScreen` 仅挂载 `AitrackerSessionPanel`，且不包含上一版 CCP 固定面板标题或布局类名。
- 类型检查、工作流测试、Vitest、Vite 构建和 Release 构建输出。

## 非目标

- 不迁入 AITracker 的 Electron/Node 运行时。
- 不改变 Rust 会话采集器、会话详情读取或外层导航。
