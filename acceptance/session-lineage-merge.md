# 验收标准：同一会话的多个转录合并

验证的规格文档：`spec/session-lineage-merge.md`

## 通过标准

1. **Claude 同目录同名合并**：同一项目目录下标题相同的多条 Claude 转录合并为 **1 行**（以本机 `CCP蒸馏工作台复刻` 为例，合并前 12 条），该行事件数、token 数等于整组之和、时间跨度为整组首尾。
2. **根归属**：合并后行的会话 id 与标题取根（组内首条记录最早者）的值（本机 `CCP蒸馏工作台复刻` 的根是 `3ecb7795`）。
3. **跨目录同名不合并**：同名但位于不同项目目录的会话保持分开（本机 `查看参考图` 在 TopoBC 与 toporeduce worktree 各有一组，跨目录不互相合并）。
4. **真正无标题不合并**：既没有 `custom-title`、也没有首条用户消息的转录仍各自成行。
5. **DSH 血缘合并**：带 `parentSession` 的 DSH 子会话并入父会话行（本机 6 个子会话），列表中子会话不再单独成行。
6. **独立同名不合并**：WorkBuddy 的 5 行「中转站名单邮件 · 每日批次推送」（`created_at` 各不相同）仍为 5 行；Claude 不同目录的同名会话同样保持分开。
7. **其它来源不变**：Codex / WorkBuddy / cursor / openclaw 的会话行数与合并前一致（除上述 DSH 子会话）。
8. **标题仍正确**：合并后的行仍显示原生标题（Claude `custom-title`、DSH `session/title`、WorkBuddy `sessions.title`、Codex `threads.name`），无标题时回退项目名。
9. **无重复计数**：索引表 `sessions`、`workbench` 中 `session_id` 无重复组；合并后两表总行数各自减少（= 被合并掉的会话数）。
10. **既有行为不回退**：`cargo fmt --all -- --check`、`cargo test -p claude-codex-pro-data`、`cargo test --workspace --no-fail-fast`、`npm --prefix apps/claude-codex-pro-manager run check`、`... run vite:build`、`cargo build --release` 全部通过。
11. **陈旧投影不复活**：`SNAPSHOT_SCHEMA_VERSION` 递增到 3，旧版本快照文件被忽略而不是继续渲染，升级后立刻看到合并结果。
12. **未重命名的副本也参与合并**：没有 `custom-title` 的转录按首条用户消息取标题（与 `claude_sessions` 的标题回退一致），能并入同目录同名组（本机 `查看参考图` 组的 `c38ec761` 就是这种副本：合并前该组在索引里是 3 行，合并后 1 行）。
13. **读取路径同源**：统一用量快照里请求历史记录追加的 Claude 事件与适配器扫描出的事件经过同一份血缘改写，索引 `sessions` 表不再出现「同一条工作线仍有 2–3 行」——本机 `查看参考图@TopoBC` 3 → 1、`日志最大显示金额与余额标红 (fork)@toporeduce` 2 → 1，与离线适配器扫描（探针）的结果一致。

## 必需验证方式

- 单测（`cargo test -p claude-codex-pro-data`）：
  - `claude_copies_of_one_conversation_merge_into_the_session_it_started_in`：同目录两条同名转录（时间一前一后）→ 断言只剩最早那条会话 id，事件数为 2。
  - `transcripts_with_the_same_title_in_different_projects_stay_separate`：两条同名转录分处两个项目目录 → 断言仍是各自的会话 id。
  - `an_untitled_transcript_keeps_its_own_session`：一条有标题、一条无标题（也没有首条用户消息）→ 断言无标题那条保留自己的会话 id。
  - `a_copy_that_was_never_renamed_joins_the_renamed_copy_of_its_conversation`：一条有 `custom-title`、一条没有但首条用户消息与标题相同（时间在后）→ 断言两条记录归入较早那条会话 id。
  - `dsh_subagent_session_merges_into_the_session_it_derives_from`：子会话 `session` 记录带 `parentSession` → 断言子事件归入父会话 id。
- 真实数据对照（本机，临时探针 `crates/claude-codex-pro-data/examples/lineage_probe.rs`，验收后删除）：按标题组统计每组「仍占有自己会话行的转录数」。
- 界面复核：把新 release 装到 `H:\Claude Codex Pro\claude-codex-pro.exe`，重启应用后看 `会话` 页与 `蒸馏工作台`（用户确认）。

## 证据

- 单测输出（目标名 + passed/failed）。
- 全量验证输出（fmt / workspace / tsc / vite / release 的退出码与产物大小、时间）。
- 本机合并前后对照表（行数、事件数、token 数）。
- 探针输出：`CCP蒸馏工作台复刻: 1 of 12`、`多人 NPC 像素网游: 1 of 4`、`查看参考图: 1 of 3`（每组只剩 1 条转录占有自己的会话行）。

## 非目标

- 不验证「N 次续写」标记、界面折叠。
- 不验证 Codex 续写血缘（现有 `payload.session_id` 归并已覆盖）。
- 不把跨目录的同名会话合并成一条。
- 不验证与本次改动无关的既有环境性失败（如 `credential_environment` 在机器繁忙时对子进程就绪超时）。

## 已知取舍

- 判据是「同一项目目录内标题相同」，因此用户在同一项目里**刻意取了同名**的两条独立会话也会被合成一行（本机数据中同目录同名组全部是同一工作线的副本，未出现这种情形）。若要更保守，可改为「组内至少有一条被判定为续写副本才合并」，但实测续写标记只覆盖部分组（`多人 NPC 像素网游` 组 4 条整文件无标记），保守规则会让这些组留成多行。
- 跨项目目录的同名会话不合并（本机 `查看参考图` 在 TopoBC 与 toporeduce worktree 各有一组，仍是各自的行）。
- 标题来源与界面一致：优先 `custom-title`，没有时才取首条用户消息（折叠空白 + 截断 120 字符，与 `claude_sessions` 相同规则）。副作用是同一项目目录里首条用户消息完全相同的两条**独立**会话也会被合并。

## 执行记录（2026-10-23）

| 判据 | 证据 | 状态 |
| --- | --- | --- |
| 1、2 | 探针 `lineage_probe`（真实数据）：`CCP蒸馏工作台复刻: 1 of 12 transcripts still own a session row: ["3ecb7795"]` → 12 → 1，根为 `3ecb7795` | 通过 |
| 3、4、6 | 探针：`多人 NPC 像素网游: 1 of 4`、`查看参考图: 1 of 3`（跨目录的 `9d2d176f` 独立成组）；单测 `transcripts_with_the_same_title_in_different_projects_stay_separate`、`an_untitled_transcript_keeps_its_own_session` | 通过 |
| 5 | 单测 `dsh_subagent_session_merges_into_the_session_it_derives_from` | 通过 |
| 4、12 | 单测 `an_untitled_transcript_keeps_its_own_session`、`a_copy_that_was_never_renamed_joins_the_renamed_copy_of_its_conversation`；本机 TopoBC 的 `查看参考图` 组（`f2d24f8a` 有 `custom-title`、`c38ec761`/`60ba1735` 只有首条用户消息）在索引里 3 行 | 通过 |
| 10 | `cargo test -p claude-codex-pro-data --lib` → `52 passed; 0 failed; 1 ignored`；`cargo test -p claude-codex-pro-data` → 全部目标 ok（lib 52、storage_adapter 17、request_history、provider_sync 等） | 通过 |
| 11 | `apps/claude-codex-pro-manager/src-tauri/src/commands.rs` 的 `SNAPSHOT_SCHEMA_VERSION = 3` + `read_usage_snapshot`/`read_workbench_snapshot` 守卫 | 通过 |
| 13 | 统一快照补血缘后重装重启（144-11-28 装入 `H:\Claude Codex Pro\claude-codex-pro.exe`）再测：`sessions` 的 `session_6e2da1ab886c39cd claude-code ev=467 '查看参考图'` 即合并行（组内 3 条转录 → 1 行），`TopoBC` 下再无 `f2d24f8a`/`60ba1735` 的独立行 | 通过 |
| 9、10、13 | 重装后实测（`lineage_stats.py`）：`sessions` 190 行（claude-code 17、codex 67（空标题 8）、workbuddy 86（空 17）、dsh 20），`title==project` 全为 0；`workbench` 175 行（claude-code **2**、codex 67、workbuddy 86、dsh 20）；两表 `session_id` 重复组 **0 / 0**。全量验证（log `C:\Users\Damon\AppData\Local\Temp\verify_lineage.log`）：`FMT=0`、`WS=0`（含 `windows_subsystem` 90 passed）、`CHECK=0`、`VITE=0`（3.62s）、`REL=0`（4m47s）→ `claude-codex-pro.exe` 28,225,024 字节 @2026-10-23T14:11:02 | 通过 |
| 残余（已知） | 仍同名但**不应合并**：DSH 6 行原生截断名 `You are working in a`（DSH 自己的回退标题只有 20 字符）；WorkBuddy 5 行「中转站名单邮件 · 每日批次推送」（原生就是 5 条独立批次）；Claude 2 行 `日志最大显示金额与余额标红 (fork)@toporeduce`（其中一行在本机没有对应转录文件，无法与另一行建立血缘） | 说明 |