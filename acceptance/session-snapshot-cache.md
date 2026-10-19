# 验收标准：会话与蒸馏工作台的持久化快照

对应规格：`spec/session-snapshot-cache.md`

## 通过标准

- [ ] 首次成功扫描后，`~/.claude-codex-pro/aitracker/cache/usage-snapshot-v1.json` 与
      `workbench-snapshot-v1.json` 存在，包含 `savedAtMs` 与页面所需数据。
- [ ] 应用重启后打开会话页/蒸馏工作台不再同步全量扫描：读取持久化快照即可返回数据；
      本机实测会话页首次读取约 0.85 秒、蒸馏工作台约 1.7 毫秒（对比扫描 70–117 秒）。
- [ ] 同一进程内连续调用命中内存缓存（微秒级）；后台刷新完成后内存缓存失效，下一次读取拿到新数据。
- [ ] 快照超过 5 分钟：读取仍立即返回旧数据，并触发一次后台刷新；不会出现并发重复扫描。
- 按文件增量：未变化文件的解析结果从缓存复用；本机实测同一批数据第二次扫描 97 秒 → 16.6 秒（request_history 36→10.4s、claude-code 适配器 24→0.3s、codex 适配器 5→0.25s、workbuddy 28→3.4s）。
- [ ] 快照缺失时：启动预热与页面命令通过同一把构建锁串行，只扫描一次；快照损坏时回退同步扫描。
- [ ] 启动钩子调用 `warm_snapshots_in_background()`，不阻塞窗口创建。

## 必需证据

- `cargo test -p claude-codex-pro-manager --lib` 通过，且包含：
  - `snapshot_files_roundtrip_and_ignore_corruption`：写入/读取一致，损坏 JSON 与缺失文件返回 None。
  - `snapshot_staleness_uses_the_refresh_window`：5 分钟窗口边界判断。
  - `file_cache::tests::cache_roundtrips_and_invalidates_on_change`：按 mtime/size 失效。
  - `file_cache::tests::corrupt_cache_file_degrades_to_empty`：缓存损坏回退为空。
  - `file_cache::tests::cache_roundtrips_and_invalidates_on_change`：按 mtime/size 失效。
  - `file_cache::tests::corrupt_cache_file_degrades_to_empty`：缓存损坏回退为空。
- 本机探针（实现期间执行，已移除）：usage 首次读 850ms / 内存 6.5µs；workbench 首次 1.7ms /
  读盘 1.7ms；扫描路径 117s / 80s。
- `cargo fmt --check`、`npm --prefix apps/claude-codex-pro-manager run check`、
  `vite:build`、`cargo build --release` 全部通过。
- 实机确认（需用户执行）：重启应用后打开会话页与蒸馏工作台，数据立即出现；等待数分钟后重新进入
  可看到刷新后的数据。

## 非目标

- 每文件增量索引、SQLite 索引库、近期优先首轮建索引（后续优化项）。
- 会话正文持久化。
