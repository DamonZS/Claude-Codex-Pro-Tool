# 验收标准：Codex 注入弹窗漫画质感 UI

验证对象：`spec/codex-injection-popup-comic-ui.md`

## 验收项

1. 规格文档存在
   - 通过标准：`spec/codex-injection-popup-comic-ui.md` 存在。

2. 验收文档存在
   - 通过标准：`acceptance/codex-injection-popup-comic-ui.md` 存在。

3. 弹窗具备漫画质感主题
   - 通过标准：注入脚本包含漫画主题标识 class 或主题 token，例如 `claude-codex-pro-comic-shell`、`comic-panel`、`comic-halftone` 等。
   - 通过标准：弹窗主容器包含粗边框、偏移阴影、网点/纸纹背景相关样式。

4. 不增删原有功能
   - 通过标准：`主页`、`推荐内容`、`支持` tab 仍存在。
   - 通过标准：`data-claude-codex-pro-setting` 开关逻辑未删除。
   - 通过标准：`data-codex-backend-status`、`data-codex-backend-repair`、服务模式按钮和支持入口仍存在。

5. 验证通过
   - 通过标准：`cargo test -p claude-codex-pro-core --manifest-path Cargo.toml --test cdp_bridge injection_script_uses_comic_modal_theme -- --nocapture` 通过。
   - 通过标准：`npm --prefix apps/claude-codex-pro-manager run check` 通过。
   - 通过标准：`npm --prefix apps/claude-codex-pro-manager run vite:build` 通过。
   - 通过标准：`cargo build -p claude-codex-pro-launcher --manifest-path Cargo.toml` 通过。
   - 通过标准：`cargo build -p claude-codex-pro-manager --manifest-path Cargo.toml` 通过。

## 不在范围内

- 不验证 Claude 中文注入样式。
- 不验证管理工具主界面整体 UI。
- 不要求修改盘古记忆数据库内容。
