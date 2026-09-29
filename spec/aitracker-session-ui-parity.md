# AITracker 会话管理界面还原

## 背景

CCP 已经接入统一的 AITRACKER 本地会话快照，但当前会话页仍使用运维面板和普通列表，和 AITracker 的会话管理界面存在明显结构差异。

## 目标

- 以 AITRACKER `SessionsPage.tsx` 的结构为基准，复刻 CCP 会话页的统计卡、筛选栏、Agent 工具标签、日期分组和会话卡片。
- 继续使用 CCP 现有 Rust/Tauri 真实采集结果、会话详情和蒸馏候选能力。
- 保留 CCP 深色透明液态玻璃材质和外层导航，不改变窗口框架。
- 会话路由默认只显示 AITRACKER 会话管理内容；上一版 CCP 的历史会话修复、Codex/Claude 双列表、独立 Agent 详情和独立会话详情面板不再作为页面固定区块渲染。

## UI / 交互要求

- 顶部显示会话数、会话工具数、对话轮次数三项统计卡。
- 搜索、近 7 天/30 天/90 天/全部、立即刷新处于同一工具行。
- Agent 工具以品牌图标标签显示，当前项使用蓝色底部高亮。
- 会话按本地日期分组，显示日期、今天/昨天标识和组内数量。
- 每条会话显示 Agent、状态徽标、项目、Provider、模型、事件/工具调用数和 Token 数，并提供恢复会话入口。
- 列表支持分页；点击会话后，在当前 AITRACKER 页面下方显示该会话的真实详情、工具调用分析和蒸馏入口。
- 窄窗口下工具行与会话卡自适应换行，内容不重叠。

## 数据要求

- 统计和列表数据继续来自 `query_aitracker_sessions` 与 `read_aitracker_capabilities`。
- 会话详情继续调用 `read_aitracker_session_detail`。
- 不添加演示数据或硬编码业务统计。

## 技术约束

- 仅修改 CCP React 页面和样式，以及本任务规格/验收文档。
- 不引入 npm 依赖，不修改 CCP 外层窗口和导航。
- AITracker 源码作为界面结构参考，不把其 Electron/Node 运行时迁入 CCP。

## 交付范围

- `apps/claude-codex-pro-manager/src/screens.tsx`
- `apps/claude-codex-pro-manager/src/workspace.css`
- 本规格与对应验收标准
