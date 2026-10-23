# 同一会话的多个转录合并（Claude 同名转录 / DSH parentSession）

## 背景

`会话` 页与 `蒸馏工作台` 的会话列表里，同一条工作线会出现多行同名会话，用户反馈为「怎么好多会话有两个一样名字的会话？」。

实测（本机 `~/.claude-codex-pro/aitracker/cache/usage-index-v1.db` 与各 Agent 原生数据）：

- 索引库里 `session_id` 重复组为 0，`(session_id, title, project)` 重复为 0 —— 没有任何一条会话被采集两次，问题不在去重。
- Claude 在**同一个项目目录**下会留下同名的多个转录：`CCP蒸馏工作台复刻` 12 条、`多人 NPC 像素网游` 4 条、`查看参考图` 3 条。它们是同一条工作线的多份副本（Claude 在上下文写满或会话被别处 resume 时另写一个转录，副本里带着继承来的对话），所以首条记录的时间戳彼此相同或首尾相接。
- 续写标记（`This session is being continued from a previous conversation…` 与 `isCompactSummary`）**不能作为判据**：CCP 组 11 条带标记，而 Otherworld 组的 4 条整文件都没有任何标记；标记出现的位置也从 107 字节到 493 KB 不等。
- 标题是可靠信号：同一工作线的副本被 Claude 写成同一条 `custom-title`（组内条数完全一致），不同工作线不会被命名成同名。
- DSH 自己记录了血缘：30 个 DSH 会话里 6 个带 `parentSession`（父会话 id）+ `delegationDepth: 1` + `origin: "subagent"`。
- WorkBuddy 的同名行（`中转站名单邮件 · 每日批次推送` 5 条）是真正独立的 5 次运行（`created_at` 各不相同、都 completed），不能合并。

结论：要合并的是「同一条工作线的多个转录」——Claude 以「同目录 + 同标题」判定，DSH 以 `parentSession` 判定。

## 目标

**包含**

- Claude：把同一项目目录（`~/.claude/projects/<slug>/`）下**标题相同且非空**的转录合并为一条会话；根（保留会话 id 与标题的那条）是首条记录时间最早的一条，其余转录改写到根。
- DSH：按 `parentSession` 把子会话递归归并进父会话。
- 合并后全部事件指向根会话 id；事件数、工具调用数、各 token 计数、时间跨度由既有聚合自然变成整组之和/首尾。
- 与「会话名对齐」既有逻辑共存：标题映射仍按（合并后的）会话 id 生效。

**不包含**

- 不做跨项目目录的同名合并：同名但位于不同项目目录的会话保持分开（WorkBuddy 每日批次、不同 worktree 的同名会话都不受影响）。
- 不做界面层的折叠/分组，不改列表排版，不展示「N 次续写」标记。
- 不解析 Codex 的续写血缘（Codex 续写文件已携带父 `payload.session_id`，现有逻辑已归并）。
- 不改 WorkBuddy / cursor / openclaw 等其它来源的采集行为。

## 用户视角

用户打开 `会话` 页或 `蒸馏工作台`：原本 12 行同名的 Claude 工作线合成 1 行（事件数、token 数、时间跨度是整组之和/首尾），名字仍是原会话名；不同项目目录下的同名会话仍是各自的行。

## 功能要求

1. Claude 分组：按转录所在**项目目录**分组；目录内标题相同的转录为一组。标题优先取 `custom-title`（trim 后非空），没有时取首条用户消息（折叠空白、截断 120 字符，与 `claude_sessions` 的标题回退一致）。
2. 根选择：组内首条记录时间戳最早者为根（时间戳缺失者排最后，其次按会话 id）；组内其余转录映射到根。
3. 既没有 `custom-title` 也没有首条用户消息的转录永不参与合并（无名会话没有被声明为同一条工作线）。
4. 标题读取：在转录头部有界窗口（1 MiB）内找第一条 `custom-title` 记录，**不在首条 user 记录处提前停止**（用户可能在会话中段才给会话命名）；同时记住首条 user 记录的消息文本作为标题回退。
5. DSH 血缘：读取会话文件首帧 `session` 记录的 `id` 与 `parentSession`；`parentSession` 存在即把该会话的事件归到父会话 id，递归到根。
6. 事件改写：命中合并的转录，其事件 `session_id` 改写为根的 raw id（之后仍由既有逻辑做哈希）。
7. 其它来源（codex / workbuddy / cursor / openclaw 等）行为不变。
8. 血缘结果按 `(root, source)` 在进程内缓存 60 秒，避免每个适配器重复扫描。
9. 读取路径同源：统一用量快照先从请求历史读到 Claude 记录、再叠加各适配器事件，两条路径必须经过同一份血缘改写；只改写适配器事件会让记录派生的行继续按未合并的会话 id 成行（表现为索引里同一条工作线仍有 2–3 行，而单跑适配器扫描只看到 1 行）。

## 数据与接口要求

- 输入：Claude 项目目录（`~/.claude/projects/**/*.jsonl`，跳过 `subagents/` 子代理目录）与 DSH 会话目录（`~/.dsh/sessions/**/session*.jsonl.zstd`）。
- 判据只依赖：Claude 的 `custom-title` 记录与每条转录首条记录的时间戳；DSH 首帧 `session` 记录的 `id`/`parentSession`。不新增外部依赖。
- 读取成本：Claude 每个候选文件只读头部（≤1 MiB，且拿到标题与首时间戳即提前结束）；DSH 只解压头部字节。不读文件尾部。
- 输出：`(source, raw_session_id) → raw_root_session_id` 的映射，供事件改写使用。

## UI / 交互要求

无界面结构改动；列表行数因此减少，行内统计与时间跨度随之变化。

## 技术约束

- 改动集中在 `crates/claude-codex-pro-data`（分组 + 事件改写）与 `apps/claude-codex-pro-manager/src-tauri/src/commands.rs`（工作台路径与统一用量快照路径的事件改写调用）。
- 不改动既有去重/归并逻辑（`SessionScope`、子代理归并、`CACHE_VERSION` 语义）。
- 不新增依赖；不改索引表结构与快照字段结构；但合并改变了投影内容，因此 `SNAPSHOT_SCHEMA_VERSION` 递增到 3（沿用既有守卫，让升级后不再渲染旧投影）。
- 保持既有测试语义：既有夹具里的转录没有同名（或没有标题），不应受影响。

## 交付范围

- `crates/claude-codex-pro-data/src/local_usage.rs`：分组构建与事件改写。
- `apps/claude-codex-pro-manager/src-tauri/src/commands.rs`：工作台路径与统一用量快照路径接入。
- 单测：`claude_copies_of_one_conversation_merge_into_the_session_it_started_in`、`transcripts_with_the_same_title_in_different_projects_stay_separate`、`an_untitled_transcript_keeps_its_own_session`、`dsh_subagent_session_merges_into_the_session_it_derives_from`。
- `spec/session-lineage-merge.md` 与 `acceptance/session-lineage-merge.md`。
- 真实数据对照（本机）：Claude `CCP蒸馏工作台复刻` 12 → 1、`多人 NPC 像素网游` 4 → 1、`查看参考图` 3 → 1；WorkBuddy 会话行数不变。

## 变更记录

- 2026-10-23：新建。
- 2026-10-23：把 Claude 的判据从「续写标记 + 时间相接的链」改为「同目录 + 同标题分组」。原因：实测续写标记只覆盖部分组（Otherworld 组 4 条整文件无标记，`isCompactSummary` 同样没有），链式规则会把这些组留成多行；而标题由 Claude 写在转录里、组内一致，是可靠且无需读文件尾部的判据。同时标题读取不再在首条 user 记录处停止（`3ecb7795` 的标题记录位于 493 KB 处，会被漏掉）。
- 2026-10-23：标题增加「首条用户消息」回退。原因：安装新版后本机仍有 4 组同名行，逐 id 回溯到文件后发现 `TopoBC` 目录的 3 条 `查看参考图` 里只有 `f2d24f8a` 带 `custom-title`，另两条（`c38ec761`、`60ba1735`）从未被重命名，`claude_head` 因此给出空标题、被排除在分组之外，界面上的名字却是会话清单按首条用户消息取的。判据与界面对齐后该组 3 行 → 1 行。
- 2026-10-23：补上「统一用量快照的记录路径也要过血缘」。原因：装回带标题回退的版本后，`会话` 页索引里 `查看参考图@TopoBC` 仍是 3 行、`日志最大显示金额与余额标红 (fork)` 仍是 2 行，而同一版本的离线适配器扫描（临时探针）只保留 1 行。逐段比对发现统一用量快照先把请求历史读到的记录直接转成事件，再叠加已过血缘的适配器事件，于是记录派生的行仍按未合并的会话 id 成行；工作台路径早已对记录调用 `session_lineage`，统一路径漏了这一步。