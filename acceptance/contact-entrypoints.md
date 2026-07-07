# 验收标准：联系我入口与联系方式展示

验证对象：`spec/contact-entrypoints.md`

## 验收项

1. 文档存在
   - 通过标准：`spec/contact-entrypoints.md` 存在。
   - 通过标准：`acceptance/contact-entrypoints.md` 存在。

2. Codex 注入弹窗新增联系入口
   - 通过标准：注入脚本包含 `data-claude-codex-pro-tab="contact"`。
   - 通过标准：注入脚本包含 `data-claude-codex-pro-panel="contact"`。
   - 通过标准：原 `home`、`recommendations`、`support` tab 仍存在。

3. QQ 群入口正确
   - 通过标准：注入脚本和管理工具关于页都包含 `10061615` 与 `1076215359`。
   - 通过标准：两个 QQ 群一键添加链接均存在。
   - 通过标准：注入弹窗链接使用新窗口打开并带 `rel="noreferrer"`。
   - 通过标准：管理工具关于页点击“一键添加”必须调用现有 `actions.openExternalUrl(...)`，由系统浏览器打开 QQ 加群链接，避免 Tauri WebView 内普通 `<a>` 点击无响应。

4. 微信二维码展示正确
   - 通过标准：用户提供的微信二维码已复制为项目资产。
   - 通过标准：注入脚本和管理工具关于页均展示该二维码。

5. 不破坏现有功能
   - 通过标准：注入脚本仍包含 `data-claude-codex-pro-setting`、`data-codex-backend-status`、`data-codex-backend-repair`。
   - 通过标准：不修改 Claude 中文注入。

6. 验证通过
   - 通过标准：注入脚本定向测试通过。
   - 通过标准：manager 关于页定向测试通过。
   - 通过标准：`npm --prefix apps/claude-codex-pro-manager run check` 通过。
   - 通过标准：`npm --prefix apps/claude-codex-pro-manager run vite:build` 通过。
   - 通过标准：`cargo build -p claude-codex-pro-launcher --manifest-path Cargo.toml` 通过。
   - 通过标准：`cargo build -p claude-codex-pro-manager --manifest-path Cargo.toml` 通过。

## 不在范围内

- 不验证 QQ 群链接在腾讯侧是否仍有效。
- 不验证扫码后的微信账号状态。
- 不验证 GitHub Release 发布。
