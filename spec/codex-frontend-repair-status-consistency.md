# Codex 前端修复状态一致性

## 背景

Codex 已通过新版启动器完成重启和脚本注入，GPT-5.6 模型也能正常选择与调用，但管理工具“修复前端连接”仍提示“未确认可注入”。现场日志证明 Codex DevTools `/json`、helper 和 renderer 心跳均已正常；最小复现进一步证明 Chromium 已返回完整 HTTP 正文，却没有立即关闭连接，现有探测器因 `read_to_string` 最终超时而误判离线。

## 目标

- 修复 Codex CDP `/json` 在线探测的假阴性。
- 让“修复前端连接”的最终提示与概览页真实在线状态一致。
- 为“响应完整但连接保持打开”的 Chromium 行为增加回归测试。

## 非目标

- 不修改 GPT-5.6 模型目录、模型请求覆盖和注入菜单。
- 不修改用户 `config.toml`。
- 不重置盘古记忆数据库或其他用户数据。
- 不为了验证而重启当前 Codex 会话。

## 功能要求

- Codex DevTools `/json` 返回 `HTTP 200` 且已收到 `webSocketDebuggerUrl` 时必须判定在线。
- 探测不得要求服务端主动关闭 HTTP 连接。
- 响应不完整、非 `HTTP 200` 或不含 `webSocketDebuggerUrl` 时仍必须判定离线。
- 修复命令继续要求真实 Codex 重启与新注入，不得重新接受修复前的旧心跳。

## 技术约束

- 最小修改 `apps/claude-codex-pro-manager/src-tauri/src/commands.rs`。
- 不引入新依赖。
- 保留现有命令名、前端按钮和返回结构。

## 交付范围

- CDP HTTP 探测修复。
- Rust 回归测试。
- 前端检查与 Manager 重建证据。
