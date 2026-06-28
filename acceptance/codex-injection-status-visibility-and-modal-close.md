# 验收标准：Codex 注入状态标识可见性与面板关闭隔离

验证对象：`spec/codex-injection-status-visibility-and-modal-close.md`

## 验收项

1. 规格文档存在
   - 通过标准：`spec/codex-injection-status-visibility-and-modal-close.md` 存在。
   - 验证证据：文件存在。

2. 验收文档存在
   - 通过标准：`acceptance/codex-injection-status-visibility-and-modal-close.md` 存在。
   - 验证证据：文件存在。

3. 注入状态文字跟随窗口文字颜色
   - 通过标准：注入脚本中状态条容器和触发按钮使用 `color: inherit`，且没有通过浅/深主题媒体查询强行覆盖标题栏标识文字颜色。
   - 验证证据：`cargo test -p claude-codex-pro-core --test cdp_bridge injection_script_exposes_left_anchored_codex_status_entry -- --nocapture` 通过。

4. 关闭按钮不触发开关
   - 通过标准：关闭按钮 click handler 包含 `preventDefault`、`stopPropagation`，遮罩 click handler 在关闭后 return。
   - 验证证据：`cargo test -p claude-codex-pro-core --test cdp_bridge injection_script_modal_close_does_not_toggle_settings -- --nocapture` 通过。

5. 定向注入脚本测试通过
   - 通过标准：`cargo test -p claude-codex-pro-core --test cdp_bridge -- --nocapture` 成功。
   - 验证证据：命令输出。

6. 前端构建通过
   - 通过标准：`npm --prefix apps/claude-codex-pro-manager run vite:build` 成功。
   - 验证证据：命令输出。

## 不在范围内

- 真实视觉截图回归。
- 重写 CCP 面板 UI。
- 改动管理工具设置页。
