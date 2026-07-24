# 验收标准：修复陈旧供应商同步锁导致历史会话修复为 0

验证对象：`spec/fix-stale-provider-sync-lock-and-status.md`

## 验收项

1. 陈旧锁恢复
   - 旧 owner 时间超过阈值时，provider sync 不再直接返回 lock skipped。
   - SQLite 中旧 `model_provider` 行和对应 rollout 能被同步到目标 provider。
2. 新鲜锁保护
   - 新鲜 provider-sync 锁仍返回 `Skipped`，不会并发写入。
3. 状态反馈
   - Tauri `sync_providers_now` 对 `Skipped` 返回失败命令结果，消息包含真实 `sync.message`。
   - 成功结果仍返回成功并持久化目标 provider。
   - 点击“修复历史会话”后，右下角在后端命令完成前显示“正在修复”运行态提示。
   - 修复成功后右下角显示修复结果和“即将重启 Codex”，并自动执行 Codex 重启。
   - 修复失败或跳过时显示对应错误，且不执行 Codex 重启。
   - 修复后的自动重启携带一次性跳过同步标志，启动器不再重复扫描历史文件；普通启动/重启不携带该标志。
   - WindowsApps 中的 `ChatGPT.exe` 被识别为官方 Codex 客户端外壳，不因进程改名而漏掉重启或注入等待。
   - 启动/重启按钮在新客户端未确认上线时显示失败原因，不出现“只关闭、不重启”的假成功。
4. 归档会话保护
   - `threads.archived = 1` 的会话保持原 provider，关联 rollout 不被改写。
   - 同一批次的未归档会话仍可修复。
5. 内存与回滚
   - `SessionChange` 不保存完整原文或完整重写文本。
   - rollout 逐文件重读和写入，备份目录保存原文件用于回滚。
   - owner PID 已退出的 Windows 锁可立即恢复。
6. 验证
   - `cargo test -p claude-codex-pro-data --test provider_sync provider_sync_recovers_stale_lock -- --nocapture`
   - `cargo test -p claude-codex-pro-data --test provider_sync provider_sync_skips_when_home_missing_or_lock_exists_and_prunes_backups -- --nocapture`
   - Manager `windows_subsystem` 同步状态契约测试通过。
   - `cargo fmt --check` 通过。

## 非目标

- 不删除会话或直接修改用户现有数据库。
- 不改变 API Key、provider schema 或会话解析格式。
