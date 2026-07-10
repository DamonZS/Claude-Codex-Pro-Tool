# 验收标准：Codex 前端修复状态一致性

验证对象：`spec/codex-frontend-repair-status-consistency.md`

## 验收项

1. 响应完整但无 EOF 时仍判在线
   - 通过标准：测试服务返回包含 `webSocketDebuggerUrl` 的完整 `HTTP 200` 响应后保持连接打开，`codex_debug_json_ready` 返回 `true`。
   - 证据：定向 Rust 单元测试。

2. 无效响应不误报在线
   - 通过标准：非 `HTTP 200` 或不含 `webSocketDebuggerUrl` 的响应返回 `false`。
   - 证据：Rust 单元测试或现有判定断言。

3. 修复结果保持真实重启约束
   - 通过标准：`repair_frontend_connection` 仍调用 Codex 重启链路并要求本次修复后的新前端心跳，不接受旧心跳。
   - 证据：`windows_subsystem` 回归测试。

4. 项目检查与构建
   - 通过标准：前端类型检查、前端生产构建、Manager 定向测试与 Manager debug 构建通过。
   - 证据：命令输出以及 `target/debug/claude-codex-pro-manager.exe` 更新时间。

## 非验收范围

- 不重新验证 GPT-5.6 上游 API 是否开放。
- 不修改或重启用户当前 Codex 会话。
