# 验收标准：GitHub Windows CI 构建失败修复

验证对象：`spec/github-windows-ci-regression.md`

## 验收项

1. Windows 回归测试通过。
   - 通过标准：`cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem -- --nocapture` 成功。
   - 证据：命令输出。

2. 概览页盘古记忆测试护栏匹配当前设计。
   - 通过标准：测试断言概览页盘古记忆卡包含开关、运行状态、Codex 注入、对话监控和 `MemoryActivityWave`。
   - 证据：源码检查和测试输出。

3. 旧入口不会被重新加回概览页。
   - 通过标准：测试断言概览页盘古记忆卡不包含 `查看/编辑经验教训`、`提炼经验教训`、`memory-overview-matrix`、`memory-overview-actions`。
   - 证据：源码检查和测试输出。

4. 前端类型检查通过。
   - 通过标准：`npm --prefix apps/claude-codex-pro-manager run check` 成功。
   - 证据：命令输出。

5. 前端生产构建通过。
   - 通过标准：`npm --prefix apps/claude-codex-pro-manager run vite:build` 成功。
   - 证据：命令输出。

6. Debug 管理工具可重新构建。
   - 通过标准：`cargo build -p claude-codex-pro-manager --manifest-path Cargo.toml` 成功。
   - 证据：命令输出与 exe 更新时间。

## 不在范围内

- 修改发布版本号。
- 修改发布产物命名策略。
- 重构盘古记忆页面。
- 回滚会话管理页或供应商页当前改动。
