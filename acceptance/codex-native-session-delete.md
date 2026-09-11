> 状态：本文件中「注入层始终关闭 sessionDelete 且不创建删除按钮」的结论已被 `spec/codex-injected-session-delete-restore.md` 取代（2026-09-11）。

# Codex 原生会话删除恢复验收

对应规格：`spec/codex-native-session-delete.md`

## 通过标准

1. `renderer-inject.js` 将 `sessionDelete` 固定为关闭状态。
2. 脚本不再调用 `installDeleteButtonEventDelegation()`，也不再创建 `codex-delete-button`。
3. 重新注入后，旧 CCP 删除按钮、确认层和右下角删除提示节点会被移除。
4. `cargo test -p claude-codex-pro-core --test cdp_bridge -- injection_script_leaves_session_deletion_to_codex_native --nocapture` 通过。

## 非目标

- 不验证 Codex 官方客户端自身的删除实现。
- 不修改用户的会话或配置数据。
