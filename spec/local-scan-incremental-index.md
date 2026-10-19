# 本地扫描增量优化与会话 SQLite 索引

## 背景

会话页与蒸馏工作台的数据来自本机会话扫描。持久化快照（`spec/session-snapshot-cache.md`）已经让页面打开不再等待，但每 5 分钟的后台刷新仍然全量重读文件：

- 冷扫描约 97–117 秒；按文件解析缓存命中后约 16.6 秒；
- 剩余耗时主要来自：H 盘 Codex rollout 的逐文件 `stat`（约 10 秒）、WorkBuddy 适配器文件遍历（约 3.4 秒）、69 MB 缓存落盘（约 2.2 秒）；
- 会话页查询目前从 60 MB JSON 快照反推（冷读约 850 ms）。

AITracker 的做法：持久化候选文件列表、每文件水位增量解析、SQLite 归一化快照供页面查询。

## 目标

1. **文件列表缓存 + 并行扫描**：候选文件列表短 TTL 缓存；请求历史的 Codex / Claude 文件解析有界并行，合并顺序与串行一致。
2. **缓存落盘节流**：文件解析缓存最多每 5 分钟落盘一次（`save_forced` 可强制），减少无意义 69 MB 重写。
3. **SQLite 会话索引**：新增独立索引库，保存会话与工作台行；会话页筛选/分页/计数走 SQL，索引缺失时回退现有快照路径。

## 非目标

- 不改变扫描器解析语义、去重、排序、截断规则。
- 不持久化会话正文。
- 不改变前端页面结构与 Tauri 命令名/返回结构。
- 不改 `ccp.db` 业务表。

## 功能要求

### 1. 文件列表缓存与并行扫描

- `cached_path_list(key, ttl, build)`：进程内 TTL 缓存候选文件列表，默认 TTL 30 秒；键包含输入路径。
- `request_history` 的 Codex rollout 列表与 Claude 文件列表经该缓存获取。
- 逐文件解析使用 `std::thread::scope` 有界并行（最多 8 个 worker），按原始文件顺序合并记录与告警，最终排序/去重/截断结果与串行一致。
- 任何线程 panic 或解析错误按原语义降级为告警，不中断整体扫描。

### 2. 缓存落盘节流

- `save()`：仅当有变更且距上次落盘 ≥ 300 秒时写入；跳过时保留 dirty 状态。
- `save_forced()`：忽略节流立即原子写入（测试与退出前使用）。
- 落盘仍使用临时文件 + rename，失败不影响扫描结果。

### 3. SQLite 会话索引

- 数据库：`~/.claude-codex-pro/aitracker/cache/usage-index-v1.db`，WAL，`user_version=1`。
- 表：`meta`、`sessions`（source/session_id 主键，含 title/project/model/provider/起止时间/事件数/工具调用数/状态/6 类 token）、`workbench`（工作台行 + rank）。
- 写入：每次扫描结束后在同一事务内全量替换对应表；失败回滚，不影响扫描与快照。
- 读取：`query_sessions` 支持 agent/status/keyword/range/page/page_size，语义与现有 `aitracker::query_sessions` 一致；`load_workbench` 按 rank 返回。
- `index_ready()` 为假时命令回退到现有快照路径；索引损坏时自动重建。
- 写入失败、索引缺失、SQL 错误均不得让页面命令失败。

## 技术约束

- Rust/Tauri 现有架构；data crate 已有 rusqlite/chrono 依赖，不新增依赖。
- 索引库与现有 JSON 快照、`ccp.db`、用户原始日志互不影响；索引可随时删除重建。
- 所有新逻辑需有单测，且不降低现有测试覆盖率。

## 交付范围

规格、验收、Rust 实现与单测、命令接线、构建验证、提交到 `main`。
