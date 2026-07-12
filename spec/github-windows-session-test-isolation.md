# GitHub Windows 会话测试隔离修复

## 背景

GitHub Actions `PR build artifacts / Windows artifacts` 在 Rust tests 阶段偶发失败，失败用例为
`commands::tests::list_local_sessions_deduplicates_threads_across_current_and_legacy_dbs`。

Manager 单元测试默认并行运行。多个测试会临时修改进程级 `CODEX_HOME`，但会话测试未与已有的路径测试锁共享同一临界区，因此可能读取其他测试的临时 Codex 目录。测试还通过手工代码恢复环境变量，发生 panic 时无法保证恢复。

## 目标

- 对所有临时修改 `CODEX_HOME` 的会话管理测试使用现有 `test_path_lock` 串行隔离。
- 复用 `TestEnvVarGuard`，在正常结束或 panic 展开时恢复原环境变量。
- 保持会话枚举、跨数据库去重与删除行为不变。

## 非目标

- 不修改生产环境的 `CODEX_HOME` 解析逻辑。
- 不修改数据库结构、会话数据或 GitHub Actions workflow。
- 不修改 Manager 前端 UI。

## 技术约束

- 仅调整测试隔离和测试夹具，不通过关闭 Rust 测试并行来掩盖竞态。
- 所有共享同一进程环境变量的相关测试必须使用同一把锁。
- 环境变量恢复必须使用 RAII，避免断言失败后污染后续测试。

## 交付范围

- `apps/claude-codex-pro-manager/src-tauri/src/commands.rs`
- `spec/github-windows-session-test-isolation.md`
- `acceptance/github-windows-session-test-isolation.md`
