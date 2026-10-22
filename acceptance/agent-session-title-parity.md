# 验收：所有 Agent 的会话名、项目标签与子代理归并对齐原生客户端

对应规格：`spec/agent-session-title-parity.md`（v2，2026-10-22）。

## 通过 / 失败标准

| # | 标准 | 判定方式 |
| --- | --- | --- |
| 1 | 同一原生会话在用量事件与标题映射里使用同一会话 id（不再并存「文件族」id） | 单测 + 本机 SQL：`workbench` 有标题的 `session_id` 在 `sessions` 表中可查到同一行 |
| 2 | `subagents/` 目录下的用量文件归并到父会话 | 单测（WorkBuddy 与 Claude 两条路径各一）；本机对照：`subagents/` 下文件产生的独立会话 id 数 = 0，`source='workbuddy'` 的 `sessions` 行数由 338 降到 154（离线复算，见执行记录） |
| 3 | Claude 全量覆盖：凡是能在 `~/.claude/projects/**/*.jsonl` 找到的会话，在 `sessions` 表中标题非空 | 本机 SQL：`source='claude-code'` 且标题非空的行数 ≥ 30（改造前 0 / 32） |
| 4 | WorkBuddy 标题不再批量回退项目名 | 本机 SQL：`workbench` 中 `title == project` 的行数 ≤ 少量真无原生标题者（改造前 223 / 279） |
| 5 | Codex 不回退 | `workbench` 中 codex 行标题非空数 ≥ 11（改造前 11 / 11）；`sessions` 中 codex 行标题非空数由 0 提升（复算 87 / 103） |
| 6 | 「会话」页与「蒸馏工作台」两条路径对同一会话标题一致 | SQL：按 `session_id` 连接 `sessions` 与 `workbench`，`title` 不等且两者均非空的行数 = 0 |
| 7 | 项目标签不出现 `unknown` | SQL：`select count(*) from sessions where project = 'unknown' or project = ''` = 0；`workbench` 同理 |
| 8 | 索引投影带标题（不再只依赖读取时兜底） | `usage_index::write_usage` 写入的 `sessions.title` 非空行数与本机标题覆盖一致；代码审查确认投影取自标题映射 |
| 9 | 用户可见效果 | 「会话」页与「蒸馏工作台」截图：同名会话显示同一标题，子代理不再出现同名空行 / `0 tokens` 行 |
| 10 | 回归 | `cargo test -p claude-codex-pro-data`、`cargo test -p claude-codex-pro-core --lib`、`cargo test --workspace --no-fail-fast`、`npm --prefix apps/claude-codex-pro-manager run check`、`npm --prefix apps/claude-codex-pro-manager run vite:build`、`cargo build --release` 全部通过 |
| 11 | 不触碰原生数据 | 运行前后抽样 `~/.claude/projects`、`~/.codex/sessions`、`~/.workbuddy/projects` 的文件 mtime 与数量不变；`git status` 只包含本规格交付范围内的文件 |
| 12 | 解析缓存版本已递增，旧规则解析出的载荷不再被复用 | `crates/claude-codex-pro-data/src/file_cache.rs` 的 `CACHE_VERSION` ≥ 2；代码审查确认缓存仍以 `(path, mtime, size)` 为键、只靠版本号失效 |

## 必需验证方式

1. 单元/集成测试（标准 1、2、3、5、7、8、10）。
2. 本机 SQL 前后对照（标准 2、3、4、5、6、7、8）：
   - `sqlite3 ~/.claude-codex-pro/aitracker/cache/usage-index-v1.db` 上执行标准里给出的 SQL。
   - 对照数据需同时给出**改造前**与**改造后**两组数字。
3. 界面证据（标准 9）：改造后重启应用，截「会话」页与「蒸馏工作台」会话列表。
4. 原生数据未被改动（标准 11）：抽样对比运行前后的文件数量与 mtime。

## 完成任务所需的证据

- 测试命令与原始输出（含新增用例名）。
- SQL 前后对照表（每个标准一行：改造前 / 改造后）。
- 改造后的界面截图与 release 产物路径、大小、构建时间。
- `git diff --stat` 与提交 hash。

## 执行记录（2026-10-22）

新增/更新用例（原始输出片段）：

- `cargo test -p claude-codex-pro-data --lib` → `test result: ok. 43 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out`，含新增 `local_usage::tests::subagent_transcripts_merge_into_their_parent_session`（WorkBuddy 子代理文件归并到父会话）、`local_usage::tests::claude_subagent_events_belong_to_the_parent_session`（Claude 子代理事件计入父会话）、`local_usage::tests::transcript_without_cwd_is_labelled_with_its_agent_name`（无 `cwd` 时回退 Agent 显示名）；DSH 采集任务合入后同命令为 46 passed / 4 ignored。
- `cargo test -p claude-codex-pro-core --lib claude_sessions` → `test result: ok. 15 passed; 0 failed; 570 filtered out`，含新增 `session_titles_cover_transcripts_the_inventory_skips`、既有 `malformed_jsonl_lines_do_not_hide_valid_records_and_subagents_are_excluded`。
- `cargo build -p claude-codex-pro-manager` 通过（仅既有 warning，无新增）。

改造前后对照：

- 改造前：现索引库 `~/.claude-codex-pro/aitracker/cache/usage-index-v1.db`（2026-10-22 之前写入）。
- 改造后：同一份代码路径的离线复算——`builtin_adapters` → `scan_adapter` → `aggregate` → `usage.sessions = collect_workbuddy_sessions` → `aitracker::project_snapshot` → 套用与 `distillation_session_titles()` 同规则的标题映射。复算脚本为临时 example，验证后已删除。

| 指标 | 改造前 | 改造后（复算） |
| --- | --- | --- |
| `sessions` 标题非空 / 行数 — claude-code | 0 / 32 | 32 / 32 |
| `sessions` 标题非空 / 行数 — codex | 0 / 11 | 87 / 103 |
| `sessions` 标题非空 / 行数 — workbuddy | 0 / 338 | 70 / 154 |
| `workbench` 中「标题 == 项目名」（workbuddy） | 223 / 279 | 0 |
| `sessions` 中 项目 = `unknown`（workbuddy） | 21 | 0 |
| `sessions` 中 `total_tokens = 0` 的行 | 124（claude 5 + workbuddy 119） | 94（claude 5 + workbuddy 89） |

已知偏差（不掩盖）：

- codex 行数 11 → 103、以及 workbuddy 行数 338 → 154，都同时受两个因素影响：①**旧解析缓存**——递增 `CACHE_VERSION` 之前，用户机上未变更的日志会持续返回旧规则产出的 `session_id`/`project`（同一脚本递增前后对比：workbuddy 334 行 / `unknown` 19 行 → 154 行 / 0 行）；②**基线库由本机正在运行的旧构建写入**（`H:\Claude Codex Pro\claude-codex-pro.exe`，2026-10-22 08:56 启动），且本机 `~/.codex/sessions` 目录为空、255 个 rollout 实际位于 `~/.codex/sessions.ccp-backup-20261013-104614-7770/`，探针经定义驱动的来源仍能读到 codex 事件，所以 codex 行数不是与索引库同口径的对比。标题覆盖比与 `unknown` 两项不受此影响。
- 复算未并入应用额外的 `request_history` 与代理请求记录，故行数与应用重启后的索引库可能有小幅差异；标题覆盖比与 `unknown` 两项不受此影响。

应用重启后复核 SQL（标准 1、2、3、4、5、6、7、8）：

```sql
select source, count(*), sum(coalesce(title,'') <> '') from sessions group by source;
select agent,  count(*), sum(coalesce(title,'') <> ''),
       sum(coalesce(title,'') = coalesce(project,'')) from workbench group by agent;
select source, count(*) from sessions where project = '' or project = 'unknown' group by source;
```

界面证据（标准 9）：待应用重启后截「会话」页与「蒸馏工作台」列表；release 产物路径与构建时间见最终交付汇报。

原生数据未被改动（标准 11）：改造前后抽样计数一致——`~/.claude/projects` `.jsonl` 167 个 / `subagents/` 目录 12 个；`~/.codex/sessions` `.jsonl` 0 个（255 个 rollout 在 `sessions.ccp-backup-20261013-104614-7770/`，非本次扫描路径）；`~/.workbuddy/projects` 2062 个文件（302 个 `.jsonl`、199 个位于 `subagents/`）、`subagents/` 目录 21 个。期间有 9 个原生文件在 20:00 之后被写入，来自各 Agent 客户端的自身运行；本次改造的代码路径全部是只读（代码审查：`local_usage.rs` / `claude_sessions.rs` / `commands.rs` 的新增逻辑只做 `read_dir`、`fs::read`、SQLite `mode=ro` 查询）。

## 已知非目标（不在本次检查范围）

- 不改用量/代币的计算方法本身；子代理归并导致的父会话用量增加是预期结果。
- 不新增 Agent 类型、不改 UI 结构、不改 Agent 原生客户端文件。
- 不追求把「原生确实没有标题」的会话造出标题；这类行允许显示项目名。
- 既有死代码、既有编译 warning 不属于本次范围。