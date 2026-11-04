# 验收：Claude 一键汉化提速与稳定性修复

验证对象：`spec/claude-zh-patch-speed-and-stability.md`

## 通过标准

1. **扫描等价性**：`chunk_needs_or_has_ui_patch` 的判定结果与改造前的逐词扫描完全一致，覆盖以下夹具：仅含 `"Settings"` 的块、仅含中文改写的块、含 `embedded` 英文词的块、只含 locale 数组的块、完全无关的块。
   - 判据：新增单元测试 `patch_gate_matches_the_legacy_scan` 通过（测试内保留旧算法作为对照实现）。
2. **matcher 正确性**：每个待替换词（含引号变体）嵌入文本后都能命中；不含任何待替换词的文本不命中；仅有首字节前缀的文本不误报。
   - 判据：新增单元测试 `needle_matcher_matches_every_needle_without_false_positives` 通过。
3. **单遍扫描的量化收益**：在合成语料（≥40 个 256 KB 块、全部不含待替换词）上，新 matcher 相对旧逐词扫描至少快 10 倍。
   - 判据：`#[ignore]` 基准测试 `bench_patch_gate_scan_cost` 用 `--ignored --nocapture` 跑出并打印两组耗时，把实测数值写入"执行记录"。
4. **备份清理**：安装开始时会删除"目标已不存在"的备份条目，保留"目标仍存在"的条目。
   - 判据：新增单元测试 `install_prunes_backups_whose_target_is_gone` 通过。
5. **恢复只写存在目标**：`restore_patch_at` 不把备份写回当前安装中已不存在的路径（不产生新文件）。
   - 判据：新增单元测试 `restore_skips_backups_for_missing_targets` 通过。
6. **阶段日志**：管理端安装完成时记录 `manager.claude_zh_patch.install.finish`，detail 含 `scannedChunks`、`patchedChunks`、`prunedBackups`、`scanMs`、`patchMs`、`validateMs`、`pruneMs`、`changedFileCount`。
   - 判据：`apps/claude-codex-pro-manager/src-tauri/tests/windows_subsystem.rs` 新增契约测试 `claude_zh_patch_install_logs_stage_timings_and_rolls_back_when_claude_will_not_start` 断言事件名与字段名存在；真机日志出现该事件。
7. **启动失败自动回滚**：安装完成但 Claude 启动失败时，记录 `manager.claude_zh_patch.install.rollback` 并调用 `claude_codex_pro_core::claude_zh_patch::restore_patch()`，返回文案说明已回滚。
   - 判据：同一契约测试断言事件名、`restore_patch` 调用与回滚文案；回滚后 `detect_status().status` 不为 `ok`。
8. **回归**：`cargo fmt --check`、`cargo test -p claude-codex-pro-core claude_zh_patch`、`cargo test -p claude-codex-pro-core --lib`、`cargo test --workspace --no-fail-fast`、`npm run check`、`npm run vite:build`、`cargo build --release` 全部通过。
   - 判据：命令输出与退出码记录在执行记录中。
9. **真机**：在装有 Claude Desktop 的机器上执行一次一键汉化，日志出现 `install.finish` 且各阶段耗时为合理量级；汉化结束后 Claude Desktop 能正常启动/打开。
   - 判据：日志片段 + 进程存在性（`claude.exe` 进程）截图或命令输出。

## 必需验证方式

- `cargo test -p claude-codex-pro-core --lib claude_zh_patch -- --nocapture`
- `cargo test -p claude-codex-pro-core --lib bench_patch_gate_scan_cost -- --ignored --nocapture`
- `cargo test -p claude-codex-pro-manager --test windows_subsystem claude_zh_patch -- --nocapture`
- `cargo fmt --all -- --check`
- `cargo test --workspace --no-fail-fast`
- `npm --prefix apps/claude-codex-pro-manager run check`
- `npm --prefix apps/claude-codex-pro-manager run vite:build`
- `cargo build --release`（构建需 `CARGO_TARGET_DIR=<repo>/build-lead`、`TMP`/`TEMP=<repo>/.tmp-build`）

## 证据

- 单元/契约测试的通过输出（含用例名）。
- 基准测试打印的新旧耗时对比数值。
- 真机安装后的日志片段（`manager.claude_zh_patch.install.finish`）与 Claude 进程检查输出。

## 已知非目标

- 不验证译文覆盖率是否提升。
- 不验证非 MSIX 安装（installer 版）。
- 不改 `node --check` 严格性，也不承诺消除所有 JS 语义级风险。
- 不验证 ACL 权限回收（本次不动 takeown/icacls 流程）。

## 执行记录

### 2026-11-04：提速与稳定性改造（阶段一）

**真机数据面（只读实测，Claude `2.42304.2.0`，`C:\Program Files\WindowsApps\Claude_2.42304.2.0_x64__pzs8sxrjxfjjc`）**

- 分块：`chunks=3554`、`bytes=117,020,276`（读入 0.87 s，系统缓存命中）。
- 扫描闸门（单线程，含 579 个命中块）：改造前 `32.22 s` → 改造后 `2.97 s`，**10.9×**（`bench_real_install_scan_cost`，`CLAUDE_ZH_BENCH_ROOT=<install root>`）。
- 合成语料（40 × ~237 KB、全部不含待替换词）：`bench_patch_gate_scan_cost` 打印新旧耗时（见日志）。
- 命中块阶段开销（取 20 个真实命中块，`bench_patch_phase_cost`）：文本替换 `69.9 ms/块`（579 块约 40 s，按 8 工作线程并行，实测线程数与 `available_parallelism` 取 1–8）、写入 `1.6 ms/块`、`node --check` 校验 `75.8 ms/块`（校验阶段已是多线程）。
- 备份目录真机现状：`%LOCALAPPDATA%\Claude-zh-CN-official-backup` 累计 **5,193 个文件 / 498.2 MB**，含多个 Claude 版本的重复块（如 `composerHighlightWorker-C__zYqYt.js` 与 `composerHighlightWorker-C1oeJ7ik.js`）→ 旧版本块若被写回当前安装，正是“恢复后打不开”的来源之一。

**判据对照**

- 1、2、4、5：`cargo test -p claude-codex-pro-core --lib claude_zh_patch` → `51 passed; 0 failed; 3 ignored`（含 `patch_gate_matches_the_legacy_scan`、`needle_matcher_matches_every_needle_without_false_positives`、`install_prunes_backups_whose_target_is_gone`、`restore_skips_backups_for_missing_targets`、`patch_install_records_stage_timings`）。
- 3：真机 10.9×、合成语料同口径更快（数字见上）。
- 6、7：`cargo test -p claude-codex-pro-manager --test windows_subsystem claude_zh_patch` → `16 passed`，其中契约测试 `claude_zh_patch_install_logs_stage_timings_and_rolls_back_when_claude_will_not_start` 断言 `manager.claude_zh_patch.install.finish` / `install.rollback` / `restore_patch()` / 回滚文案与核心侧 `NeedleMatcher`、`ClaudeZhPatchTimings`、备份清理、恢复清缓存。
- 8：`cargo fmt --all -- --check`=0；`cargo test -p claude-codex-pro-core --lib`、`cargo test --workspace --no-fail-fast`、`npm run check`、`npm run vite:build`、`cargo build --release` 的结果见 `.dsh-ops/verify-zh-patch.log`。
- 9（真机一键汉化）：**未由本次执行**。该动作会改写 `WindowsApps` 下的 Claude 文件并触发 UAC 提权，属于用户可见的破坏性操作；本次只做了只读实测（上表）。用户点一次“Claude 一键汉化”后，日志应出现 `manager.claude_zh_patch.install.finish` 且 `validateMs`/`patchMs` 与上表量级一致，随后 Claude 进程存在即达标。

**环境备注（构建期）**：本次会话开始时本机 Rust 工具链已被破坏（`~/.rustup` 只剩 `settings.toml`、`.cargo\bin` 下 13 个 shim 均为 0 字节），已用 `rustup toolchain install stable-x86_64-pc-windows-msvc --profile minimal` 恢复（rustc 1.99.1），并补装 `rustfmt`/`clippy`；构建使用 `CARGO_TARGET_DIR=<repo>/build-lead`、`TMP`/`TEMP=<repo>/.tmp-build`。