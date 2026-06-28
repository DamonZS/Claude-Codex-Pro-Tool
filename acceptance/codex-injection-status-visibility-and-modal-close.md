# 验收标准：Codex 注入状态标识可见性与面板关闭隔离

验证对象：`spec/codex-injection-status-visibility-and-modal-close.md`

## 验收项

1. 规格文档存在
   - 通过标准：`spec/codex-injection-status-visibility-and-modal-close.md` 存在。
   - 验证证据：文件存在。

2. 验收文档存在
   - 通过标准：`acceptance/codex-injection-status-visibility-and-modal-close.md` 存在。
   - 验证证据：文件存在。

3. 注入状态文字使用截图同款窗口文字色
   - 通过标准：注入脚本中状态条容器、盘古记忆标识使用 `#a9a4a9`，计数文字继承该颜色，且没有通过浅/深主题媒体查询强行覆盖标题栏标识文字颜色。
   - 验证证据：`cargo test -p claude-codex-pro-core --test cdp_bridge injection_script_exposes_left_anchored_codex_status_entry -- --nocapture` 通过。

4. 关闭按钮不触发开关
   - 通过标准：关闭按钮 click handler 包含 `preventDefault`、`stopPropagation`，遮罩 click handler 在关闭后 return。
   - 验证证据：`cargo test -p claude-codex-pro-core --test cdp_bridge injection_script_modal_close_does_not_toggle_settings -- --nocapture` 通过。

5. CCP 面板主题与管理工具一致
   - 通过标准：注入脚本中 CCP 面板包含 `#ffffff`、`#172033`、`#64748b`、`#0f766e`、`#dce3ed` 和 `border-radius: 8px` 等管理工具浅色运维面板关键色值。
   - 验证证据：`cargo test -p claude-codex-pro-core --test cdp_bridge injection_script_exposes_left_anchored_codex_status_entry -- --nocapture` 通过。

6. CCP 面板不显示用户脚本管理入口
   - 通过标准：注入脚本中不存在 `data-claude-codex-pro-tab="userScripts"`、`data-claude-codex-pro-panel="userScripts"`、`data-codex-user-scripts-*`、`data-codex-user-script-*` 和打开面板时的 `/user-scripts/list` 请求。
   - 验证证据：`cargo test -p claude-codex-pro-core --test cdp_bridge injection_script_modal_hides_user_scripts_management -- --nocapture` 通过。

7. 定向注入脚本测试通过
   - 通过标准：`cargo test -p claude-codex-pro-core --test cdp_bridge -- --nocapture` 成功。
   - 验证证据：命令输出。

8. 前端构建通过
   - 通过标准：`npm --prefix apps/claude-codex-pro-manager run vite:build` 成功。
   - 验证证据：命令输出。

## 不在范围内

- 真实视觉截图回归。
- 删除用户脚本后端路由或用户本地脚本数据。
- 改动管理工具设置页。
