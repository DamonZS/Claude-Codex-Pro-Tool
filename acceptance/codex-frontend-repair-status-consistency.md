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

## 补充验收：CDP 端口释放竞态

- 通过标准：首选 CDP 端口可绑定时，强制重启继续使用首选端口。
- 通过标准：首选 CDP 端口在等待期内持续不可绑定时，强制重启使用测试指定的可用备用端口。
- 通过标准：`LaunchRequest` 与上线等待链使用相同的实际 CDP 端口。
- 通过标准：状态文件仍保留旧端口时，旧状态不能被接受，但本次请求端口的有效 CDP 直探结果仍能完成上线确认。
- 验证证据：Manager Rust 单元测试与 `windows_subsystem` 契约测试通过。

## 补充验收：renderer 心跳启动代际

- 通过标准：心跳虽在 45 秒内，但时间早于当前 `LaunchStatus.started_at_ms` 时，Manager 不得显示 `codex_injected = true` 或 `frontend_runtime_online = true`。
- 通过标准：心跳时间等于或晚于当前启动时间且状态有效时，仍能确认注入。
- 验证证据：Manager Rust 单元测试覆盖旧心跳拒绝和本次心跳接受。

## 补充验收：注入控制台稳定连接

- 通过标准：测试 bridge 先收到一个被刻意阻塞的慢请求、再收到 `/backend/status` 时，状态请求须在慢请求释放前完成回写。
- 通过标准：重复执行 `build_bridge_script` 不会重新创建已有 callback `Map`，也不会把已有请求序号归零。
- 通过标准：注入脚本会清理旧 backend heartbeat、建立新代际，并拒绝旧代际异步检查更新当前 UI。
- 通过标准：一次失败保留最近成功状态；连续失败达到阈值才显示“未连接”；下一次成功立即恢复“已连接”。
- 通过标准：`/backend/status` 的 bridge 与 helper 探测并行启动，任一通道先返回成功时立即显示“已连接”；修复请求不并行重复执行。
- 通过标准：页面重新可见时立即发起状态检查，watchdog 周期与 renderer 的 5 秒心跳错开。
- 验证证据：`cdp_bridge` 定向测试、core 测试、前端检查和生产构建通过。
- 运行证据：结束旧 Manager/launcher 后全量构建到默认 `target/release`，启动新 Manager，并连续检查实际 renderer 日志不再出现排队造成的 `backend_bridge_timeout`。
