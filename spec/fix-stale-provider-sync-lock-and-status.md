# 修复陈旧供应商同步锁导致历史会话修复为 0

## 背景

供应商切换后，Codex 会按 `threads.model_provider` 过滤历史会话。管理工具的“历史会话修复”负责把旧 provider 元数据同步到当前 provider。同步使用目录锁防止并发，但进程崩溃后旧锁会永久残留，后续每次修复都直接跳过并返回 0；Tauri 命令还把 `Skipped` 包装成成功提示，用户无法看到真实原因。

## 目标

- 陈旧的 provider-sync 锁可以自动恢复，继续执行带备份的历史会话同步。
- 新鲜锁仍阻止并发同步，不能破坏正在运行的修复。
- 内部 `Skipped` 结果在 UI 中显示失败状态和具体原因，不再伪装成“已完成 0 条”。
- 已归档或已删除的会话不得被同步到当前 provider，也不得改写其 rollout 元数据。
- 用户点击“修复历史会话”后，右下角必须立即显示进行中提示；修复成功后显示结果并明确告知即将自动重启 Codex。
- 仅在修复状态真正成功时自动重启 Codex，使修复后的索引立即生效；跳过或失败不得重启。
- 修复成功后的自动重启不得再次执行 Provider Sync，避免连续两次扫描全部历史文件导致 Codex 长时间卡顿；普通启动和手动重启仍保留启动时同步。
- Windows 官方客户端外壳从 `Codex.exe` 改名为 `ChatGPT.exe` 后，启动器仍须识别其 WindowsApps 进程并完成重启、CDP 等待和注入。
- “启动/重启 Codex”只有在新启动记录及 CDP/后端端口确认上线后才返回成功；启动器提前退出或超时必须返回失败，不能只报告已关闭旧进程。
- 扫描不得同时把全部 rollout 的原文和重写副本保存在内存；全文备份使用磁盘，应用按单文件处理。
- 不改变 SQLite 更新条件、rollout 内容、备份或 provider 选择语义。

## 技术要求

- provider-sync 锁写入 owner 时间；超过明确阈值且 owner 文件无效或已陈旧时，安全移除锁目录并重试。
- Windows 上 owner PID 已退出时立即视为陈旧锁，不必等待时间阈值。
- 锁目录只允许移除精确的 `tmp/provider-sync.lock`，不递归处理其父目录。
- 同步成功继续持久化目标 provider；同步跳过不得持久化选择。
- Tauri `sync_providers_now` 根据 `ProviderSyncStatus` 返回成功或失败命令结果，并保留 `syncMessage`。
- 任何 SQLite 数据源标记为 `archived` 的 thread 都视为非修复目标；关联 rollout 同样跳过。
- 前端在发起同步前展示 `running` Toast 并等待一次绘制；成功结果 Toast 包含“即将重启 Codex”，随后调用现有 Codex 重启流程。
- 修复成功触发重启时传递一次性 `skipProviderSync`，管理工具将其转发为静默启动器参数；该参数只影响本次启动，不持久化到设置。
- 待改 rollout 在写入前复制到 provider-sync 备份目录；失败时从磁盘备份恢复。

## 交付范围

- provider-sync 锁恢复逻辑及测试。
- Tauri 同步状态映射及契约测试。
- 本规格与验收标准。
