# 概览状态、修复反馈与 Codex 注入锚点回归修复

## 背景

概览页近期恢复 `App.tsx` 后，用户反馈盘古记忆总览没有显示新增开关与运行态，点击“修复前端连接”没有可见反馈，Codex 窗口中的注入状态标识仍与对话标题重叠，没有以窗口左侧菜单“帮助”作为同行锚点。

这些问题会让用户无法判断当前运行状态，也无法确认修复动作是否真正执行，并且 Codex 顶部注入标识遮挡标题会影响正常阅读。

## 目标

本次修复包含：

- 概览页必须显示盘古记忆状态卡、盘古记忆开关、运行态、Codex/Claude 注入态、对话监控波形和数据库信息。
- “修复前端连接”“修复后端服务”“刷新 Claude 第三方配置”点击后必须立即显示运行中反馈，并在命令完成后显示结果。
- Codex 注入状态标识定位必须优先使用顶部菜单中的“帮助/help”作为锚点，避免优先选中左侧导航或对话标题。
- 本地调试可执行文件使用的前端构建产物必须更新，避免运行旧前端导致页面看不到源码改动。

本次不包含：

- 重新设计概览页整体布局。
- 改动供应商、插件市场或 Claude 第三方网关核心逻辑。
- 删除或重置用户本地配置、登录状态、记忆数据库。

## 用户视角描述

用户打开管理工具概览页时，应能直接看到 Codex、Claude 和盘古记忆三个状态块的真实分段状态。点击修复按钮后，右下角应马上出现“正在修复”的提示，而不是长时间无反应。打开 Codex 窗口后，顶部状态标识应与菜单“帮助”在同一行定位，不再压到当前对话标题。

## 功能要求

- `Codex 状态` 分段显示运行、注入、前端在线、后端在线。
- `Claude 状态` 分段显示运行、汉化注入、前端注入、Inspector/CDP 状态。
- `盘古记忆` 状态卡使用与 Codex/Claude 一致的分段状态。
- `盘古记忆总览` 显示启用开关、运行消息、注入状态、对话监控波形、长期记忆、待确认、工作区、数据库和最近备份。
- 修复类按钮必须先渲染运行中 toast，再执行后端命令。
- 后端命令结果必须通过 toast 显示，失败不能静默吞掉。
- 修复前端连接命令必须有命令级硬超时；如果 Codex 或 Claude 注入探测卡住，必须在超时后返回 failed/degraded 结果和具体卡住原因，不能让概览页一直停留在“正在重新检查并注入”。
- Codex 注入定位必须先查找顶部菜单“帮助/help”，只在找不到时才回退到左侧导航或其他可见元素。
- 修复前端连接与修复后端服务必须分别返回 Codex 和 Claude 的独立状态，不能用任意一侧成功冒充整体成功。
- Claude Node Inspector 只能作为主进程 Inspector 状态，不能被当成前端页面 CDP 端口；但当没有页面 CDP、只有 Node Inspector 时，修复命令应尝试通过 Electron 主进程 `BrowserWindow.webContents.executeJavaScript` 注入 Claude 前端。
- 修复前端连接对 Codex 旧启动记录必须先验证 CDP `/json` 和 helper `/backend/status` 当前在线；CDP 离线时不能继续在旧端口上等待强制刷新超时，必须返回明确原因并提示需要重新启动 Codex 注入入口。
- 本地 helper 端口被占用时必须验证 `/backend/status` 来自 Claude Codex Pro helper，不能把未知进程占用端口当作成功。
- 修复后端服务遇到 helper 端口被旧 Claude Codex Pro 进程占用、但 `/backend/status` 没有响应时，必须尝试终止旧的本项目进程并重新启动 helper；若占用者不是本项目进程或无法恢复，必须保留失败并记录占用进程诊断信息。
- Claude 一键开发模式写入完成后，必须同步启动并验证本地模型映射代理 `127.0.0.1:57331/backend/status`，不能在代理尚未监听时仅返回“已写入”。
- 修复后端服务的 helper 状态等待必须使用异步探测，避免在 Tauri/Tokio runtime 中用阻塞 socket 抢占线程，导致刚启动的 helper 没机会响应。
- 本地端口不能假设固定可用。Codex helper / Chat 协议代理首选 `57321`，Claude Desktop 本地模型映射代理首选 `57331`，但启动前必须探测端口；若首选端口被其他本机服务占用且不是可验证的 Claude Codex Pro helper，必须选择备用可用端口，不能误报成功。
- Claude Desktop 第三方开发模式写入 profile 前必须使用实际启动并验证的 Claude 本地代理端口，`inferenceGatewayBaseUrl` 不能继续写死 `57331`。
- Codex Chat 协议代理启用时不能覆盖掉已选择的可用 helper 端口，避免端口选择器避让后又回退到固定 `57321`。

## UI / 交互要求

- 盘古记忆状态分段使用现有 `status-segment` 样式。
- 盘古记忆开关使用现有 `ToggleSwitch`。
- 对话监控波形使用现有 `memory-activity-wave`。
- 修复按钮文案保持：
  - `刷新 Claude 第三方配置`
  - `修复前端连接`
  - `修复后端服务`

## 数据与接口要求

- 前端使用已有 Tauri 命令：
  - `refresh_claude_third_party_config`
  - `repair_frontend_connection`
  - `repair_backend_service`
  - `load_memory_assist_status`
- 不新增远程网络依赖。
- 不记录 API Key、Bearer Token 或完整授权配置。

## 技术约束

- 保持现有 React/Tauri/Rust 架构。
- 不引入新依赖。
- 最小改动相关文件。
- 必须重建 `apps/claude-codex-pro-manager/dist`，确保 debug exe 加载最新前端。

## 交付范围

- `apps/claude-codex-pro-manager/src/App.tsx`
- `assets/inject/renderer-inject.js`
- `crates/claude-codex-pro-core/tests/cdp_bridge.rs`
- `spec/overview-memory-injection-repair-regression.md`
- `acceptance/overview-memory-injection-repair-regression.md`

## 补充要求：Claude 开发模式状态

- 概览页初始加载必须检查 Claude 一键开发模式状态。
- 如果 Claude 第三方配置已经写入，Claude 一键开发模式卡片必须显示“已写入”。
- 该检查是只读状态同步，不应重新写入 Claude 配置。
