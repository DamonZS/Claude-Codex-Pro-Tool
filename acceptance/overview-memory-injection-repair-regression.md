# 验收标准：概览状态、修复反馈与 Codex 注入锚点回归修复

验证对象：`spec/overview-memory-injection-repair-regression.md`

## 验收项

1. 规格文档存在
   - 通过标准：`spec/overview-memory-injection-repair-regression.md` 存在。
   - 验证证据：文件存在检查。

2. 验收文档存在
   - 通过标准：`acceptance/overview-memory-injection-repair-regression.md` 存在。
   - 验证证据：文件存在检查。

3. 概览页盘古记忆总览已补回
   - 通过标准：源码中存在 `盘古记忆开关`、`对话监控`、`memory-activity-wave`、`toggleMemoryAssistEnabled`。
   - 验证证据：源码检查和前端类型检查。

4. 修复按钮有即时反馈
   - 通过标准：修复类动作在调用 Tauri 命令前先设置 `running` toast，并等待一帧渲染。
   - 通过标准：`repair_frontend_connection` 后端命令对 Codex 和 Claude 注入探测都有硬超时；超时必须返回 failed/degraded 和详情，不能让前端永久停留在 running toast。
   - 通过标准：Codex CDP 端口离线时，`repair_frontend_connection` 不继续调用强制注入等待超时，而是返回旧端口已离线的详情。
   - 通过标准：Claude 只有 Node Inspector 端口时，`repair_frontend_connection` 会尝试通过 Electron 主进程 `BrowserWindow.webContents.executeJavaScript` 注入前端，而不是直接判定不可注入。
   - 验证证据：源码检查和前端类型检查。

5. Codex 注入锚点优先使用帮助菜单
   - 通过标准：注入脚本中 `findCodexWindowLeftAnchor` 在扫描左侧导航前先调用 `findCodexHelpAnchor`。
   - 验证证据：`cdp_bridge` 定向测试断言。

6. 前端构建产物已更新
   - 通过标准：`npm --prefix apps/claude-codex-pro-manager run vite:build` 成功。
   - 验证证据：命令输出。

7. 类型与定向测试通过
   - 通过标准：
     - `npm --prefix apps/claude-codex-pro-manager run check` 成功。
     - `cargo test -p claude-codex-pro-core --test cdp_bridge -- --nocapture` 成功。
     - `cargo build -p claude-codex-pro-manager` 成功。
   - 验证证据：命令输出。

8. 修复后端服务能处理 helper 端口冲突
   - 通过标准：`ensure_detached_helper` 在端口已被可验证的本项目旧进程占用时会尝试恢复；无法验证为本项目 helper 时不能误报成功。
   - 通过标准：`/backend/status` 启动后能返回 `transport = "http-helper"`。
   - 验证证据：`cargo test -p claude-codex-pro-core --test launcher detached_helper -- --nocapture`。

9. Claude 第三方代理写入后必须验证在线
   - 通过标准：`configure_claude_desktop_dev_mode` 是异步命令，并在写入 profile 前先启动或验证 Claude 本地模型映射代理，随后使用实际端口等待 `/backend/status`。
   - 通过标准：`wait_helper_backend_online` 使用异步 TCP 探测函数，不能直接调用阻塞版 `helper_backend_online`。
   - 验证证据：`cargo build -p claude-codex-pro-manager`。
10. 本地端口冲突时必须避让并写入实际端口
   - 通过标准：Claude 本地模型映射代理首选 `57331`，但端口被未知本机服务占用时必须选择备用可用端口，不能把未知服务当成已在线 helper。
   - 通过标准：Claude 一键开发模式与供应商写入的 `inferenceGatewayBaseUrl` 必须使用实际代理端口，测试中可传入备用端口并断言 profile 写入该端口。
   - 通过标准：Codex Chat 协议代理启用时继续使用 `select_helper_port` 返回的实际端口，不能强制覆盖为 `57321`。
   - 验证证据：`cargo test -p claude-codex-pro-core --test launcher protocol_proxy_port -- --nocapture`、`cargo test -p claude-codex-pro-core --test claude_desktop_provider proxy_port -- --nocapture`、`cargo test -p claude-codex-pro-core --test plugin_hub claude_desktop_dev_mode_profile_writes_custom_proxy_port -- --nocapture`。

## 不在范围内

- 手动验证用户本机已经关闭旧管理工具实例。
- 修改用户本地盘古记忆数据库内容。
- 修改 Codex 登录、Claude 登录或第三方供应商密钥。
## 补充验收：Claude 开发模式状态

- 通过标准：概览页 `refreshRoute("overview")` 会调用 `refreshClaudeDesktopDevMode(true)`。
- 通过标准：已写入 Claude 第三方配置时，概览页 Claude 一键开发模式卡片显示“已写入”。
- 验证证据：`cargo test -p claude-codex-pro-manager --test windows_subsystem -- --nocapture`。
