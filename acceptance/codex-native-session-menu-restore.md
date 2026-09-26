# Codex 原生会话菜单验收

对应规格：`spec/codex-native-session-menu-restore.md`

## 通过标准

1. 注入设置中 `sessionDelete`、`markdownExport`、`projectMove` 固定关闭，且 CCP 控制舱不再显示这三个会话行增强开关。
2. 会话扫描不调用 `/session-availability`，不根据 `codexDeletedSessions` 隐藏原生会话。
3. 启动/扫描清除旧 CCP 会话操作组、归档导出按钮、确认层、移动弹层和删除事件监听器。
4. 注入脚本不再创建 `.codex-session-actions`、删除按钮或更多菜单。
5. `node --check assets/inject/renderer-inject.js` 通过。
6. `cargo test -p claude-codex-pro-core --test cdp_bridge injection_script_restores_native_session_menu -- --nocapture` 通过。

## 未覆盖

- 真实运行中的 Codex 窗口需要重启 CCP/重新注入后手动检查右键菜单；构建不会自动刷新已运行进程。
