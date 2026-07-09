# 跨平台公共发布与配置隔离审查验收标准

对应规格：`spec/cross-platform-release-privacy-review.md`

## 通过标准

1. Windows installer 打包输入只来自 `dist/windows/app` 中的 release exe，不包含 `%APPDATA%`、`%LOCALAPPDATA%` 的用户配置目录。
2. Windows ZIP 打包输入只来自 `dist/windows/app/*`。
3. macOS DMG/ZIP 打包输入只来自 `dist/macos/stage` 的 `.app` bundle 和 Applications 链接，不包含 `~/Library/Application Support`、`~/.codex`、`~/.claude`。
4. Release workflow 不上传 `settings.json`、`relay*.json`、`*.toml` 用户配置、`memory_assist.sqlite`、`auth.json`、`credentials`、`*.env`。
5. 供应商/API key 只在运行时用户目录读写，不作为构建产物来源。
6. 诊断报告测试继续证明设置中的密钥会脱敏。
7. `scripts/release/verify-release-workflow.js` 覆盖 release 资产与隐私隔离合同。
8. 定向 Rust workflow/installer 测试通过。
9. `npm --prefix apps/claude-codex-pro-manager run check`、`cargo fmt --check`、`git diff --check` 通过。

## 证据要求

- 结构测试输出。
- 本地 release workflow 合同脚本输出。
- 前端类型检查输出。
- 格式和 diff 检查输出。
