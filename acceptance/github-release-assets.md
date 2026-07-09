# GitHub Release 构建与资产发布验收标准

对应规格：`spec/github-release-assets.md`

## 通过标准

1. `auto-release-installers.yml` 的 Release Notes 模板包含“更新内容”和“验证”。
2. 自动 Release Notes 仍带 `auto-release-installers-managed` 标记，避免触发手动 release-assets 工作流重复构建。
3. Windows job 上传：`windows-x64-setup.exe` 与 `windows-x64.zip`。
4. macOS matrix 每个架构上传：`${arch}.dmg` 与 `${arch}.zip`。
5. `latest.json` 上传仍存在，并且生成逻辑排除 `latest.json` 自身。
6. 工作流中有 9 个 Release 资产的说明/断言：6 个构建资产 + latest.json + GitHub Source code zip/tar.gz。
7. 回归测试 `github_auto_release_workflow_builds_installers_with_v0_tags` 通过。
8. `npm --prefix apps/claude-codex-pro-manager run check` 通过。
9. `git diff --check` 通过。

## 非目标检查

- 不要求本地实际运行 GitHub hosted macOS/Windows runner。
- 不要求本地创建 GitHub Release。
- 不要求修改应用业务 UI。

## 证据要求

- 定向 Rust 测试输出。
- 前端类型检查输出。
- diff 检查输出。
