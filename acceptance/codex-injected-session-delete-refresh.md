# 注入会话删除后的列表刷新验收

对应规格：`spec/codex-injected-session-delete-refresh.md`

## 通过标准

1. `removeDeletedRow()` 仍调用 `row.remove()`，并在同一成功路径调用 `window.location.reload()`。
2. `removeDeletedRow()` 不再包含 `shouldReload` 或仅针对当前会话的条件刷新。
3. 删除成功流程调用 `removeDeletedRow(row, button)`。
4. 脚本包含 `codexDeletedSessions` tombstone，删除成功后写入，会话扫描通过 `filterDeletedSessionRows` 隐藏对应旧行。
5. 撤销结果为 `undone` 时清除对应 tombstone 并刷新列表。
6. `MutationObserver` 监听 `data-app-action-sidebar-thread-id`，覆盖虚拟列表复用行。
7. 后端只返回同时满足 `threads` 记录、非空 `rollout_path` 和 rollout 文件存在的本地会话。
8. 后端扫描发现的多个 SQLite 数据库，并兼容 `state_5.sqlite`。
9. 后端查询失败或超时时不隐藏原列表。
10. `node --check assets/inject/renderer-inject.js` 通过。
11. `cargo test -p claude-codex-pro-data --test storage_adapter available_session_ids -- --nocapture` 通过。
12. `cargo test -p claude-codex-pro-core --test bridge_routes bridge_routes_cover_all_current_paths -- --nocapture` 通过。
13. `cargo test -p claude-codex-pro-core --test cdp_bridge injection_script_filters_deleted_sessions_after_codex_reprojects_rows -- --nocapture` 通过。
14. 完整 `cdp_bridge` 若存在失败，必须确认失败测试与本任务无关且没有新增回归。

## 验证方式

- 使用 `cdp_bridge` 的源码契约测试检查注入脚本实际编译内容。
- 使用 Node 语法检查确认脚本可解析。
- 如重新构建，检查 `target/release/claude-codex-pro.exe` 内含刷新关键片段。

## 手动验收

重新构建并重新启动或重新注入后，删除一个非当前会话；确认该行立即消失，页面刷新完成后仍不再出现，且点击其它会话不再访问已删除 rollout。

## 非目标

- 不验证 Codex 官方客户端自身的删除实现。
- 不删除或修改用户会话数据之外的本地配置。
