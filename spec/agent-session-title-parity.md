# 规格：所有 Agent 的会话名、项目标签与子代理归并对齐原生客户端

## 背景

用户反馈（截图 + 说明，2026-10-22）：「蒸馏工作台」会话列表和「会话」页里，成批会话显示的是**项目名**（如 `TopoBC`、`toporeduce`）而不是各 Agent 原生客户端里的**会话标题**。

用户明确要求：

1. 「不止 WorkBuddy，所有的 agent 会话都是如此」——修复必须面向所有已接入 Agent，不得只特判 WorkBuddy。
2. 子代理产生的会话，口径为**归并到父会话**（用户裁决 A）。
3. 一并修掉项目名落到 `unknown` 的行。
4. Claude 一侧的问题同样要覆盖（Claude 同样存在成批会话显示项目名的情况）。

本机实测（2026-10-22，`~/.claude-codex-pro/aitracker/cache/usage-index-v1.db` + 原生数据目录）：

| 数据面 | codex | claude-code | workbuddy |
| --- | --- | --- | --- |
| `sessions` 表（会话页数据源） | 11 行，标题全空 | 32 行，标题全空 | 338 行，标题全空 |
| `workbench` 表（蒸馏列表数据源） | 11 行，10 行有标题 | 10 行，10 行有标题 | 279 行，57 行有标题 |
| 其中「标题 == 项目名」 | 1 | 0 | 223 |
| 原生标题来源实际覆盖 | 11 / 11 | **10 / 32** | **57 / 279** |

根因（四条，均已在源码与本机数据上验证）：

1. **会话 id 派生不统一**。用量事件里带原生会话 id 时走 `session_id_from_structured(source, raw_id)`（`crates/claude-codex-pro-data/src/local_usage.rs:219-221`）；取不到时回退 `session_id_from_relative_file(source, identity)`（同文件 `:223-225`，把相对路径哈希成 id）。标题映射只按 `structured` 族建键，于是「文件族」的行永远匹配不到标题，前端回退 `session.title || session.project` → 显示项目名。
2. **各 Agent 的标题来源覆盖面小于用量扫描覆盖面**。Claude 一侧，标题映射只遍历 `list_claude_sessions()` 的清单（本机 10 条），而用量扫描会读遍 `~/.claude/projects/**/*.jsonl`（167 个文件、32 个会话 id）→ 22 条会话永远拿不到标题。WorkBuddy 一侧同理：DB `sessions.title` 覆盖 57 条，其余 222 条回退项目名。
3. **子代理 / 续写文件被当成独立会话**。本机布局实测：
   - WorkBuddy：`.workbuddy/projects/<项目>/<父会话id>/subagents/agent-<子id>.jsonl`（21 个 `subagents/` 目录，子文件内 `sessionId` 是子代理自己的 id）→ 产生无标题、`tokens=0` 的伪会话行。
   - Claude：`.claude/projects/<项目>/<父会话id>/subagents/agent-<子id>.jsonl`（12 个目录，子文件内 `sessionId` 已是**父**会话 id）→ 该路径本身能归并，但同一目录下另有续写/fork 的 `<uuid>.jsonl`，其 `sessionId` 是自身 id 且清单未收录 → 无标题。
   - Codex：`~/.codex/sessions/YYYY/MM/DD/rollout-<时间>-<父id>_<子id>.jsonl`，文件内 `payload.session_id` 是**父** id → 现有实现已能归并。
4. **索引投影不带标题**。「会话」页走 SQLite 索引时，`commands.rs:7110-7112` 用 `aitracker::project_snapshot(&usage)` 投影写入，投影里没有标题；而 `workbench` 行由 `commands.rs:7898-7917` 构建时带标题。读取路径只能靠 `apply_session_titles()`（`commands.rs:8666-8668`）兜底，一旦映射不覆盖该行，索引路径就会显示项目名——这正是「同一个会话在蒸馏列表有标题、在会话页却是项目名」的来源。

## 目标

- 包含（对**所有已接入 Agent** 一致生效：codex、claude-code、workbuddy，以及其它已接入且提供会话标题的 Agent）：
  1. **统一会话 id 派生**：只要记录能取到原生会话 id，一律用 `structured(raw_id)`；文件路径只作为**归并后的父会话 id** 来源，不再产生第二族 id。
  2. **子代理/续写文件归并到父会话**：`subagents/` 段取该段前一级目录名作为原生会话 id；归并后的事件（含其 tokens）计入父会话，父会话因此出现的用量增长是本规格**预期结果**，不是需要规避的副作用。
  3. **标题来源覆盖全量**：标题映射覆盖每个 Agent 的**全部**会话 id（包含清单未收录、仅存在于用量扫描中的会话），而不是只覆盖原生清单的子集；Claude 一侧必须覆盖全部会话文件。
  4. **两条路径标题一致**：「会话」页（SQLite 索引路径）与「蒸馏工作台」（快照路径）对同一会话显示同一标题；索引投影写入时即带标题，读取时的 `apply_session_titles()` 作为兜底保留。
  5. **项目标签不再出现 `unknown`**：优先记录 `cwd` 的 basename；取不到时回退 Agent 显示名。
  6. **可验证**：新增/更新 Rust 用例覆盖 id 族统一、`subagents` 归并、Claude 全量标题覆盖、项目标签回退；并给出本机前后的行级对照证据。
- 不包含：不新增 Agent 类型；不改任何 Agent 的原生数据文件；不改 UI 结构、列与交互；不做与本问题无关的重构；不动用量/代币的**计算方法**（`tokens` 字段本身的算法不变，变化只来自「哪些事件算同一个会话」）。

## 用户视角描述

1. 打开「会话」页：每一行的名称都是该会话在 Codex / Claude Code / WorkBuddy 原生客户端里看到的名字；项目名只出现在副标题。
2. 打开「蒸馏工作台」会话列表：同上；子代理产生的事件计入父会话，不再出现同名空行或 `0 tokens` 的伪会话。
3. 极少数确实没有原生标题的会话，显示项目名（而非空白）。
4. 没有任何一行显示 `unknown`。

## 功能要求

1. `local_usage` 的会话 id 派生：相对路径中存在 `subagents` 段时，raw id 取该段**前一级目录名**（父会话 id），仍按 `structured` 族派生；其余情况保留现有行为。
2. 归并后的事件按父会话归属 `session_id` / `project` / `model`；子文件取不到 `cwd` 时沿用可推导的项目段，不产生新的 `unknown` 行。
3. 标题映射 `distillation_session_titles()`（`apps/claude-codex-pro-manager/src-tauri/src/commands.rs:7610-7652`）覆盖所有已接入 Agent 的原生标题源：
   - Codex：`threads.id → COALESCE(name, title)`（现有）。
   - WorkBuddy：`sessions.id → COALESCE(custom_title, title)`（现有），并对 `subagents/` 归并的行命中父会话标题。
   - Claude：`~/.claude/projects/**/*.jsonl` 的**全部**会话（含清单未收录者），标题取该会话可读标题（既有 `claude_sessions` 的标题提取逻辑），并覆盖 `subagents/` 归并到父会话。
4. 「会话」页读取路径：索引行的标题与快照行一致；`usage_index::write_usage` 写入的 `sessions` 投影带标题（新增参数或等价改造），读取时仍保留 `apply_session_titles()` 兜底。
5. 项目标签：`project_label` 对绝对路径取 basename；取不到 `cwd` 的行（WorkBuddy 的 DB 会话行、无 `cwd` 的 transcript 事件）回退 Agent 显示名（`WorkBuddy` / `Codex` / `Claude Code`），禁止输出 `unknown`。不从文件路径推项目段：缺 `cwd` 的 DB 行没有对应文件，而有文件的事件行已带 `cwd`。
6. 去重与稳定：同一会话的多次扫描结果幂等；`session_<20 hex>` 对外格式不变。
7. 解析缓存失效：`file_cache::CACHE_VERSION` 递增（2026-10-23 为 3）。缓存以 `(path, mtime, size)` 为键，解析规则变化若不递增版本，用户机器上所有**未变更**的日志会继续以旧规则解析出的 `session_id` / `project` 命中缓存，本次修复（含功能要求 8 的文件名回退）在这些文件上不可见。

8. **文件名即会话 id 的回退**：文件名本身携带原生会话 id 的转录（WorkBuddy 的 `<workspace>/<原生会话 id>.jsonl`；这类文件里只有部分 record kind 重复会话 id，甚至完全没有），取不到原生会话 id 时用**文件 stem** 走 `structured` 族派生。仅当 stem 形如原生 id（长度 ≥ 16、只含 `[0-9a-f-]`、至少含一个数字）时才这样回退，其余仍按相对路径哈希——避免把 `agent-child`、`session-parent` 这类语义文件名当会话 id。
9. **持久化投影快照的版本守卫**：应用写出的投影快照（`~/.claude-codex-pro/aitracker/cache/usage-snapshot-v1.json`、`workbench-snapshot-v1.json`）必须带 schema 版本；版本与当前构建不一致（含旧文件缺该字段）时**视为不存在**，由本次进程重新采集。理由：快照的键是 `(path, mtime, size)` 之外的整个投影结果，升级后旧进程写下的行会在内存与磁盘两层继续渲染（内存 TTL 600s + 陈旧判定窗口 5 分钟）——本机实测升级后仍显示约 10 分钟的项目名行。

## UI / 交互要求

- 列表行名称 = 标题，副标题 = 项目名（现有结构不变）；不新增列、按钮或筛选项。
- 标题为空时前端仍回退项目名，不显示空白。

## 数据与接口要求

- 会话 id 仍是对外稳定的 `session_<20 hex>`；归并只改变「哪些事件算同一个会话」，不改变单个会话内部的去重键（`event` 私密 id 仍含会话 id + 事件身份）。
- 不改变 `AitrackerSnapshot` / `LocalUsageSnapshot` 的 JSON 结构（标题已在既有字段里）。
- `usage_index` 的 `sessions` 表结构不变，只改变写入值。

## 技术约束

- 数据层改动集中在 `crates/claude-codex-pro-data/src/local_usage.rs`；命令层改动集中在 `apps/claude-codex-pro-manager/src-tauri/src/commands.rs`；Claude 标题来源若需复用既有提取逻辑，可读取 `crates/claude-codex-pro-core/src/claude_sessions.rs` 的公开接口，不复制实现。
- 不得改动 Agent 原生数据；不得改 `file_cache` 的键与失效规则（仍为 `(path, mtime, size)`），但**必须**递增 `CACHE_VERSION`，用它作为解析规则变更的显式失效手段；不得新增依赖。
- 既有 `#[cfg(test)]` 测试必须保持通过。

## 交付范围

- `crates/claude-codex-pro-data/src/local_usage.rs`（及数据 crate 内既有测试模块）。
- `crates/claude-codex-pro-data/src/file_cache.rs`（仅 `CACHE_VERSION` 递增）。
- `apps/claude-codex-pro-manager/src-tauri/src/commands.rs`（标题映射 / 索引标题一致性）。
- 需要时的 `crates/claude-codex-pro-data/src/usage_index.rs`（`write_usage` 标题投影）。
- 本规格与对应验收文档。
- 验证证据：Rust 测试、`npm run check`、`vite:build`、`cargo build --release`，以及本机前后行级对照。

## 变更记录

- 2026-10-22：新建（v1），承接「所有 Agent 会话名对齐原生客户端」。
- 2026-10-22：按用户更正重写为 v2。修正三处偏差：①范围明确覆盖**所有** Agent 并对 Claude 单列根因与覆盖要求（v1 误记「Claude 无需额外归并」）；②把「子代理归并导致的用量归属变化」由矛盾的不包含项改为**预期结果**（用户已裁决归并到父会话）；③把标题覆盖面从「原生清单」提高到「全量会话」，并新增索引投影带标题与 `unknown` 项目标签的判据。
- 2026-10-22：v3。离线复算本机数据时发现既有解析缓存让修复对未变更文件不可见（旧缓存里仍带旧规则产出的 `session_id` / `project`），因此新增功能要求 7（递增 `file_cache::CACHE_VERSION`）并把交付范围补上 `file_cache.rs`；同时把「不得改 file_cache 语义」精确为「不改键与失效规则，但必须递增版本」。
- 2026-10-22：v4。把功能要求 5 与目标 5 中「其次文件路径中的项目段」删掉：缺 `cwd` 的两类行分别是 WorkBuddy 的 DB 会话行（没有文件可推项目）与无 `cwd` 的 transcript 事件（有文件时也已带 `cwd`），多一层路径推断只会增加复杂度而没有任何一行会用到它；实测 `unknown` 已由「回退 Agent 显示名」清零。
- 2026-10-23：v5。用户复查真实应用后仍有「很多会话以项目名为名」，实测定位到两个 v4 未覆盖的来源，新增功能要求 8、9：①WorkBuddy 的 `<uuid>.jsonl` 转录缺 `sessionId` 时只按相对路径哈希，永远匹配不到按会话存的标题（本机 86 条可定位行中 56 条因此丢失标题）；②升级前进程写下的投影快照会被新构建继续渲染（本机实测旧行在升级后仍显示约 10 分钟）。