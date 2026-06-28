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
   - 通过标准：Codex 前端在线状态同时支持 `frontend_runtime_online` 或 CDP 在线，不能只依赖 `debug_port_online`。
   - 通过标准：盘古记忆状态读取近期 `renderer.memory_runtime` / `renderer.script_loaded` 心跳，并在 CDP 不可用但运行时心跳新鲜时显示 `Codex 已注入` 和 `等待会话变化` / `对话监控运行中`。
   - 通过标准：Claude 状态中的 `CDP 受阻` 不会在 Claude 进程运行时把 Claude 状态卡主状态判定为失败。
   - 通过标准：`StatusActionTile` 使用 `status-segment-list` / `status-segment` 渲染 `已写入`，与其它状态卡片标签对齐。
   - 验证证据：源码检查和前端类型检查。

4. 修复按钮有即时反馈
   - 通过标准：修复类动作在调用 Tauri 命令前先设置 `running` toast，并等待一帧渲染。
   - 通过标准：`repair_frontend_connection` 后端命令只对 Codex 注入探测设置硬超时；超时必须返回 failed 和详情，不能让前端永久停留在 running toast。
   - 通过标准：Codex CDP 端口离线时，`repair_frontend_connection` 不继续调用强制注入等待超时，可以自动终止旧 Codex/launcher 并通过新版启动器重启 Codex，随后返回成功、降级或失败详情。
   - 通过标准：`repair_frontend_connection` 不再尝试通过 Claude 页面 CDP、Node Inspector 或中文包装窗口注入 Claude 前端。
   - 通过标准：官方 Claude Desktop 没有可用前端调试端口时，只显示诊断状态；不得自动打开 `Claude localization` / Claude 中文包装窗口。
   - 通过标准：不得修改 `assets/inject/claude-chinese-inject.js` 的中文注入、翻译表或 DOM 翻译逻辑。
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

8. Codex 注入弹窗主题与管理工具一致且不显示用户脚本
   - 通过标准：`assets/inject/renderer-inject.js` 中 CCP 弹窗使用管理工具同款浅色运维面板色系，包括 `#ffffff` 面板、`#172033` 主文字、`#64748b` 辅助文字、`#0f766e` 主按钮/开关色、`#dce3ed` 边框和 `8px` 圆角。
   - 通过标准：CCP 弹窗不再渲染 `用户脚本` 标签、`data-claude-codex-pro-panel="userScripts"` 面板、`data-codex-user-scripts-*` 或 `data-codex-user-script-*` 控件，也不在打开弹窗时请求 `/user-scripts/list`。
   - 通过标准：不删除后端用户脚本路由和本地用户脚本数据；本次只移除 Codex 注入弹窗中的用户脚本管理入口。
   - 验证证据：`cargo test -p claude-codex-pro-core --test cdp_bridge -- --nocapture`。

9. 修复后端服务能处理 helper 端口冲突
   - 通过标准：`ensure_detached_helper` 在端口已被可验证的本项目旧进程占用时会尝试恢复；无法验证为本项目 helper 时不能误报成功。
   - 通过标准：`/backend/status` 启动后能返回 `transport = "http-helper"`。
   - 验证证据：`cargo test -p claude-codex-pro-core --test launcher detached_helper -- --nocapture`。

10. Claude 第三方代理写入后必须验证在线
   - 通过标准：`configure_claude_desktop_dev_mode` 是异步命令，并在写入 profile 前先启动或验证 Claude 本地模型映射代理，随后使用实际端口等待 `/backend/status`。
   - 通过标准：`wait_helper_backend_online` 使用异步 TCP 探测函数，不能直接调用阻塞版 `helper_backend_online`。
   - 验证证据：`cargo build -p claude-codex-pro-manager`。
11. 本地端口冲突时必须避让并写入实际端口
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

## 补充验收：Codex 盘古记忆注入状态一致

- 通过标准：`claude-codex-pro.exe` 注入到 Codex 的桥接运行时必须实现 `/memory/status`、`/memory/session`、`/memory/search`、`/memory/learn`、`/memory/candidates`、`/memory/approve`、`/memory/reject`、`/memory/selfcheck`，不能返回“盘古记忆尚未接线”的默认实现。
- 通过标准：管理工具的“修复前端连接”不能把 `renderer.memory_runtime` 中 `runtime.status = "failed"` 的心跳当成前端修复成功；应继续执行 Codex bridge 强制重注入。
- 通过标准：概览页状态卡片中的状态标签应与卡片标题同一行显示，`Claude 状态` 与 `Claude 一键开发模式` 不得把标签压到下一行。
- 验证证据：`cargo check -p claude-codex-pro-launcher --manifest-path Cargo.toml`、`cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem -- --nocapture`、`npm --prefix apps/claude-codex-pro-manager run check`、`npm --prefix apps/claude-codex-pro-manager run vite:build`、`cargo build -p claude-codex-pro-launcher --manifest-path Cargo.toml`、`cargo build -p claude-codex-pro-manager --manifest-path Cargo.toml`。
