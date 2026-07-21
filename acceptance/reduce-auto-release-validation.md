# 验收标准：减少 GitHub 自动发布重复验证

验证对象：`spec/reduce-auto-release-validation.md`

## 验收项

1. 自动发布移除重复质量校验。
   - 通过标准：`auto-release-installers.yml` 不包含 `npm run check` 和 `cargo test --workspace`。
   - 证据：工作流契约测试和源码检查。

2. 自动发布保留产物必需构建。
   - 通过标准：Windows 和 macOS job 保留 `npm run vite:build`；Windows 保留 `cargo build --release`；macOS 保留两个目标架构的 release build。
   - 证据：工作流契约测试。

3. 自动发布保留产物验证和发布行为。
   - 通过标准：Windows installer/ZIP、macOS DMG/ZIP、bundle/plist/codesign、资产数量、上传、`latest.json`、发布和失败草稿清理逻辑保持存在。
   - 证据：现有工作流契约断言通过。

4. PR Build 保留完整质量门禁。
   - 通过标准：`pr-build.yml` 仍包含 `npm run check`、`npm run vite:build`、`cargo test --workspace` 和 release build。
   - 证据：新增契约断言通过。

5. Release Notes 与实际验证一致。
   - 通过标准：说明前端生产构建和平台产物验证，不再声称自动发布执行 TypeScript 检查或 workspace 测试。
   - 证据：契约断言和源码检查。

6. 定向回归测试通过。
   - 通过标准：`cargo test -p claude-codex-pro-manager --test windows_subsystem github_auto_release_workflow_builds_installers_with_v0_tags -- --exact --nocapture` 成功。
   - 证据：命令输出。

7. 修改内容格式有效。
   - 通过标准：`cargo fmt --check` 与 `git diff --check` 成功，并使用 YAML 解析器解析工作流成功。
   - 证据：命令输出。

## 不在范围内

- 本地模拟 GitHub hosted Windows/macOS runner。
- 创建或发布真实 GitHub Release。
- 修改 PR Build 的验证范围。
- 修改应用功能或 UI。
