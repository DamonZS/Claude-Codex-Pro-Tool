# 验收标准：本地扫描增量优化与会话 SQLite 索引

对应规格：`spec/local-scan-incremental-index.md`

## 通过标准

### 文件列表缓存与并行扫描

- [ ] `file_cache::cached_path_list` 在 TTL 内命中、过期后重建，并有单测覆盖。
- [ ] `request_history` 的 Codex / Claude 候选文件列表使用该缓存（TTL 30 秒）。
- [ ] 逐文件解析并行执行，合并顺序确定；数据 crate 与 manager crate 现有测试全部通过。
- [ ] 本机实测：同一批数据第二次扫描不慢于串行版本，且结果条数/内容一致（记录数、事件数不变）。

### 落盘节流

- [ ] `save()` 在 300 秒内不重复写 69 MB 缓存；`save_forced()` 立即写。
- [ ] 单测覆盖：强制写入成功后文件存在；节流窗口内普通 `save()` 不改变文件；窗口外或强制写入生效。

### SQLite 会话索引

- [ ] `usage-index-v1.db` 在首次扫描后生成；`meta.sessions_written_at_ms` 存在。
- [ ] `query_sessions` 的分页、总数、agent/status/keyword/range 过滤语义与现有内存实现一致，并有单测覆盖。
- [ ] `workbench` 表可写入并按 rank 读回，有单测覆盖。
- [ ] 索引缺失时 `index_ready()==false` 且命令回退现有路径；索引文件损坏时自动重建，不 panic。
- [ ] 会话页命令在索引就绪时走 SQL 路径，返回结构不变；实测查询耗时为毫秒级。

## 必需证据

- `cargo test -p claude-codex-pro-data --lib` 0 failed（含 `file_cache`、`usage_index` 新增测试）。
- `cargo test -p claude-codex-pro-manager --lib` 0 failed。
- `cargo fmt --check`、`npm --prefix apps/claude-codex-pro-manager run check`、
  `npm --prefix apps/claude-codex-pro-manager run vite:build`、`cargo build --release` 全部通过。
- 提交记录：变更已提交到 `main` 并推送到 `origin/main`，提交信息包含详细中文摘要。

## 非目标

- 会话正文索引、跨机器同步、索引数据迁移到 `ccp.db`。
- 修改前端页面结构或 Tauri 命令契约。
