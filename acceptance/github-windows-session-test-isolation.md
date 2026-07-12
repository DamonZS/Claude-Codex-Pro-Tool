# 验收标准：GitHub Windows 会话测试隔离修复

验证对象：`spec/github-windows-session-test-isolation.md`

## 验收项

1. **失败用例通过**
   - `list_local_sessions_deduplicates_threads_across_current_and_legacy_dbs` 单独运行成功。

2. **共享环境变量已隔离**
   - 三个修改 `CODEX_HOME` 的会话管理测试均持有 `test_path_lock`。
   - 三个测试均通过 `set_test_codex_home` 返回的 RAII guard 恢复环境变量，不保留手工恢复逻辑。

3. **Manager 单元测试并行回归通过**
   - `cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --lib -- --nocapture` 成功。
   - 为提高对偶发竞态的检出率，至少重复运行一次 Manager lib 测试。

4. **格式检查通过**
   - `cargo fmt --check` 成功。

## 不在范围内

- 修改生产会话行为、数据库结构或用户数据。
- 修改 GitHub Actions workflow 或关闭测试并行。
- 修改前端 UI。
