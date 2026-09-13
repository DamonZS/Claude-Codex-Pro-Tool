# 注入会话删除后的列表刷新验收

对应规格：`spec/codex-injected-session-delete-refresh.md`

## 通过标准

1. `removeDeletedRow()` 仍调用 `row.remove()`，并在同一成功路径调用 `window.location.reload()`。
2. `removeDeletedRow()` 不再包含 `shouldReload` 或仅针对当前会话的条件刷新。
3. 删除成功流程调用 `removeDeletedRow(row, button)`。
4. `node --check assets/inject/renderer-inject.js` 通过。
5. `cargo test -p claude-codex-pro-core --test cdp_bridge injection_script_refreshes_session_list_after_delete -- --nocapture` 通过。
6. `cargo test -p claude-codex-pro-core --test cdp_bridge` 的结果不新增由本任务引起的失败。

## 验证方式

- 使用 `cdp_bridge` 的源码契约测试检查注入脚本实际编译内容。
- 使用 Node 语法检查确认脚本可解析。
- 如重新构建，检查 `target/release/claude-codex-pro.exe` 内含刷新关键片段。

## 手动验收

重新构建并重新启动或重新注入后，删除一个非当前会话；确认该行立即消失，页面刷新完成后仍不再出现，且点击其它会话不再访问已删除 rollout。

## 非目标

- 不验证 Codex 官方客户端自身的删除实现。
- 不删除或修改用户会话数据之外的本地配置。
