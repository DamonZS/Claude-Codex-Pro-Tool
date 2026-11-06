# 验收：安全写入冲突放弃

对应 `spec/live-write-conflict-guard.md`。

1. `cargo test -p claude-codex-pro-core --lib live_write` 全绿，至少覆盖：
   - 磁盘内容等于 `expected` 时写入成功；
   - 磁盘内容被改后拒绝写入，且磁盘仍是外部内容；
   - `expected=None` 且文件不存在时写入成功；文件已出现时拒绝；
   - 写入后 `<文件名>.<数字>.<数字>.tmp` 孤儿被清掉，同目录不相关的 `notes.tmp` 保留。
2. 冲突错误满足 `live_write::is_conflict(&error) == true`，文案包含 `被其它程序改动` 与目标文件路径。
3. `relay_config.rs` 中 `set_codex_goals_feature_in_home` 的落盘调用点只有 `live_write::write_bytes_if_unchanged`，不再直接 `settings::atomic_write`。
4. `relay_config.rs` 中 `write_codex_live_atomic` 的 auth.json / config.toml 两次写入都走 `live_write::write_bytes_if_unchanged`。
5. `write_codex_live_atomic` 的 config 回滚分支只在 `!live_write::is_conflict(&error)` 时调用 `restore_optional_file`。
6. `cargo test -p claude-codex-pro-core --lib relay_config`（或等价定向测试）全绿，既有 relay 配置行为不变。
7. `cargo fmt --all -- --check` 退出码 0。
8. `cargo build --release --package claude-codex-pro-manager` 退出码 0，产物可装到 `H:\Claude Codex Pro`（真机切换供应商不出错）。
9. 现有备份语义不变：`create_live_backup` 仍先生成 `home/backups/claude-codex-pro-live-<ms>/`，再写文件。