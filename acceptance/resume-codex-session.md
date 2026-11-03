# 会话页「恢复会话」真实恢复 验收

对应规格：`spec/resume-codex-session.md`

## 通过标准

1. `cargo test -p claude-codex-pro-data --lib local_usage` 通过，含：
   - `codex_resume_targets_map_rollout_files_and_index_names`：夹具
     `.codex/sessions/2026/11/03/rollout-2026-11-03T12-27-26-<uuid>.jsonl` 与
     `.codex/session_index.jsonl`（`id` + `thread_name`）→ 映射 key 为
     `private_id("codex","structured",<uuid>)`、value 含 uuid 与 thread_name；
   - 文件名不含 UUID 的 rollout 被跳过，不产生条目。
2. `cargo test -p claude-codex-pro-manager --test windows_subsystem` 通过，含：
   - `resume_session_command_launches_codex_resume`：`commands.rs` 内含
     `resume_agent_session` 命令与 `app_paths::find_codex_desktop_cli()` 调用，
     且 `lib.rs` 的 invoke_handler 注册该命令；
   - `resume_button_invokes_the_real_resume_command`：`screens.tsx` 的卡片按钮文案为
     「恢复会话」且 `onClick` 调用 `resume_agent_session`；禁用条件绑定 `resumeId`；
     只读入口文案为「查看会话」，两者不再是同一个 handler。
3. `npm --prefix apps/claude-codex-pro-manager run check`（tsc）通过。
4. `npm test`（cwd `apps/claude-codex-pro-manager`）通过。
5. `cargo fmt --check` 通过。
6. 反例保护：`resume_agent_session` 在无法解析目标时返回 `ok=false`，
   且不调用 `read_aitracker_session_detail`、不打开蒸馏视图。

## 必需验证方式

- 单元/契约测试：上列命令的真实输出。
- 手工验证（需本机运行态）：在会话页对一张 Codex 会话点击「恢复会话」→
  新控制台出现 `codex resume <uuid>` 会话，输入一条消息可继续对话；
  对一张无本地记录的会话，按钮为禁用态且提示原因。
- 真机验证前必须记录 `codex.exe` 路径与 `session_index.jsonl` 行数（只读取证）。

## 验证证据

- 命令与结果记入 `dev/changelog.md`；日志落在 `.dsh-ops/`（已 gitignore）。

## 非目标

- Claude/WorkBuddy/DSH 会话恢复。
- Codex 桌面应用内会话定位。
- 网关 `cc12h` 计费表达式（网关侧）。
- 历史会话 provider 改写后的界面刷新（见 `acceptance/fix-provider-id-stability.md`）。

## 执行记录（2026-11-03 22:5x）

环境约束：本会话只允许写工作区内的路径（`%TEMP%`、`D:\Project`、`H:\Claude Codex Pro` 均写拒绝），
因此 `tempfile::tempdir()` 只能落在仓库内；构建/测试统一使用
`CARGO_TARGET_DIR=D:/Project/Claude-Codex-Pro-Tool/build-lead`、
`TMP=TEMP=D:\Project\Claude-Codex-Pro-Tool\.tmp-build`。

### 已通过

| 判据 | 命令 | 结果 |
| --- | --- | --- |
| 1（数据映射） | `cargo test -p claude-codex-pro-data --lib codex_resume_targets` | 2 passed / 0 failed：`codex_resume_targets_map_rollout_files_and_index_names`、`usage_index_round_trips_codex_resume_targets` |
| 索引持久化 | `cargo test -p claude-codex-pro-data --lib usage_index` | 7 passed / 0 failed（含版本断言与 resume 往返） |
| 2（契约） | `cargo test -p claude-codex-pro-manager --test windows_subsystem` | 见下节全量结果 |
| 3（tsc） | `npm --prefix apps/claude-codex-pro-manager run check` | 0（Duplicate identifier `Play` 已修） |
| 5（fmt） | `cargo fmt --all -- --check` | 0 |

### 实现补充（本轮自查发现并修复）

索引库原先没有 resume 列，`会话`页在索引路径下拿到的 `resumeId` 恒为空 → 按钮永远禁用。
已在 `crates/claude-codex-pro-data/src/usage_index.rs`：
`SCHEMA_VERSION` 1 → 2；`sessions` 表新增 `resume_id` / `resume_name`（`TEXT NOT NULL DEFAULT ''`）；
`SESSION_INSERT_SQL` 补 `?20/?21`；`SESSION_COLUMNS` 与 `session_from_row`（`row.get(17/18)`）同步；
测试里 `PRAGMA user_version` 断言改用 `SCHEMA_VERSION`；新增往返用例
`usage_index_round_trips_codex_resume_targets`。版本号变更会让旧缓存库被丢弃重建，
下一次扫描后 `resume_id` 即落库（`~/.claude-codex-pro/cache/usage-index-v1.db` 可用字符串检索验证）。

### 环境性失败（与本改动无关，已在 HEAD 中存在且未被本 diff 触碰）

- `cargo test -p claude-codex-pro-data --lib` → 53 passed / **1 failed** / 1 ignored：
  `local_usage::tests::worktree_path_groups_by_layout_when_repository_is_gone`
  （`left: Some("Claude-Codex-Pro-Tool")` vs `right: Some("LostRepo")`）。该用例把临时目录放在
  仓库内，`canonical_project_label` 因此解析到本仓库根目录名；`local_usage.rs` 的 diff 只新增
  `codex_resume_targets` 与其夹具，未触碰该函数。
- `cargo test -p claude-codex-pro-core --lib` → 3 failed：`settings::tests::atomic_write_applies_private_windows_acl`、
  `settings::tests::write_private_file_applies_windows_acl_before_atomic_replace`、
  `ccpbot::tests::log_tail_returns_the_last_lines`；三个文件在工作区均**未修改**（`git status` 不含）。
- `npm test`（apps/claude-codex-pro-manager）→ 失败于写 `node_modules/.vite-temp`（EPERM）。

### 未完成

- 手工点击验证（判据 6 的反例与真机点击）：需用户在运行中的 CCP 上点一次「恢复会话」，
  观察 `codex resume <uuid>` 打开对应会话；本轮已单独验证 CLI 形态（`codex resume` 接受 UUID 或会话名）。

### 真机数据面验证（2026-11-03 23:06–23:08）

换装 `29,402,112 B @23:05:05` 并启动 Manager（PID 37156）后，应用重写了索引库
（`~/.claude-codex-pro/aitracker/cache/usage-index-v1.db`，`sessions_written_at_ms=1793718383837`）。
只读拷出该库查询（探针 `.dsh-ops/probe_resume_index.py`）：

- `source='codex'` 行数 **8**，其中 `resume_id<>''` **8**（例：`session_7b2a8106bb33d67627c4` →
  `01a1a12b-8ad2-7a13-a0c6-4d2679550886`；`session_e5ab37c6e045ec1119ab` →
  `01a19fa9-7a9c-7793-8506-2c30d71db149` + `resume_name='统一品牌布局与自定义API'`）；
- 其他 source（`claude-code`/`workbuddy`/`dsh`）带 `resume_id` 的行数 **0** —— 只有 Codex 会话可恢复，
  符合规格。
- 会话页读的就是这个索引（`index_ready()` 为真时走 `usage_index::query_sessions`），
  因此「恢复会话」按钮对上述 8 张 Codex 会话为可用态；`resume_id` 为空的会话保持禁用并显示原因。
- 说明：上面 UUID/标题在 bash 控制台回显为乱码属控制台编码，库内为 UTF-8（应用内显示正常）。