# 安全写入：落盘前重读比对（冲突放弃）

## 背景

Codex / Claude 会自己改写 `~/.codex/config.toml`、`~/.codex/auth.json` 这类文件。
CCP 现有落盘走 `settings::atomic_write`（临时文件 + `sync_all` + 替换），只保证"旧内容或新内容"，
不保证"我们读到的内容还在"：读到 A、算完新内容时磁盘已变成 B，直接写就会静默覆盖 B。

借用 cc-switch `src-tauri/src/live/engine.rs` 的乐观冲突检测思路：写之前重读一次，
内容和调用方读到的不一致就放弃这次写入，把决定权交回用户（重试即可）。

## 功能要求

1. 新增 `write_bytes_if_unchanged(path, bytes, expected)`：落盘前重读 `path`；重读内容与 `expected` 一致才写，否则返回 [`Conflict`] 错误，磁盘内容保持原样。
2. `expected` 为 `None` 表示"调用方读到文件不存在"：文件此时确实不存在才写；文件已被别人创建则同样放弃。
3. `Conflict` 是可识别的具体错误（`live_write::is_conflict(&anyhow::Error)`），调用方能据此决定回滚策略。
4. 冲突错误文案说明"本次写入已放弃（磁盘内容未被覆盖）"，并带上文件路径。
5. 写入沿用 `settings::atomic_write`，不新写第二套落盘实现（备份、权限收紧等语义保持一致）。
6. 写入成功后清理同目录下本功能的孤儿临时文件（`<文件名>.<数字>.<数字>.tmp`），其它 `.tmp` 不动。
7. `relay_config::set_codex_goals_feature_in_home`（读-改-写）与 `relay_config::write_codex_live_atomic`（auth.json / config.toml）改走该入口。
8. `write_codex_live_atomic` 失败回滚时，若失败原因是冲突，不得用旧字节覆盖 `config.toml`（否则等于替对方的改动做回退）。

## 非目标

- 不做跨进程文件锁（cc-switch 的 `AppWriteGuard` 只覆盖它自己的写入方）。
- 不收窄"读后长时间规划"的窗口：`write_codex_live_atomic` 的 `expected` 取的仍是落盘前几行读到的字节，
  它只能挡住这几行到落盘之间的改动；由更早读取派生的 `config_text` 不在本次范围。
- 不做冲突 UI（提示用户手工选择保留哪份内容）。

## 技术约束

- 只新增 `crates/claude-codex-pro-core/src/live_write.rs`，不引入新依赖。
- 临时文件命名规则由 `settings::temp_path_for` 决定，清理逻辑必须与之一致（纯数字段）。
- 已有 `write_codex_live_atomic` 的调用方与错误文案保持兼容（错误链里追加 `context`）。

## 变更记录

- 2026-11-06：新增 `live_write` 模块（冲突检测 + 孤儿临时文件清理），接入 `set_codex_goals_feature_in_home` 与 `write_codex_live_atomic`，并让回滚路径识别冲突。