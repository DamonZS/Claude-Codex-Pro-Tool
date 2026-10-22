# 本机 Codex 跨磁盘归档故障修复

## 背景与证据
桌面版 26.1007.2314.0 的 thread/archive 多次返回 -32603、Windows os error 17。sessions 是 C 盘普通目录，archived_sessions 是指向 H:\C-Offload\codex-archived_sessions 的目录联接；失败记录仍为 archived=0，源 rollout 位于 C 盘。

## 目标
- 只读诊断目录、指定失败记录及日志，使用临时文件复现跨盘原子移动失败。
- 准备默认仅预览的本机维护脚本，将 sessions 复制到 H:\C-Offload\codex-sessions，逐文件校验 SHA-256 后，在 C 盘保留原目录备份并建立 sessions 目录联接。
- 让普通会话和现有归档目录位于同一实际磁盘，继续使用 Codex 原生归档操作。
- 回滚仅在 Codex 关闭、目标及备份文件均未变化时执行；保留 H 盘副本。

## 授权与技术约束
- 当前阶段仅诊断、脚本准备及隔离目录验证。实际数据迁移和关闭 Codex 待用户明确确认。
- Apply 在当前真实 Codex Home 下要求 Codex 桌面及 CLI 进程已关闭；默认预览不修改目录或数据库。
- 不修改状态数据库、模型配置、原生客户端二进制或 CCP 注入代码。
- 目标存在、目录漂移、嵌套重解析点或文件校验失败时停止，保留源目录及复制出的数据。
- 所有路径使用 LiteralPath；备份目录位于同一个明确的 Codex Home 下；只移除经过核对的目录联接本身。

## 交付
本地诊断报告、预览结果、PowerShell 维护脚本、受控 C/H 跨盘测试与回滚证据，存于 docs/task-evidence/archive-failure/。本次维护不触发软件发布。
## 用户已确认与后台交接
用户明确同意关闭 Codex 并迁移会话，随后询问关闭后由谁主持迁移。采用当前用户的 Windows 任务计划程序托管一次性独立 PowerShell 进程；已验证服务进程祖先链、临时数据完整迁移、归档移动及任务删除后的子进程存活。实际后台进程先报告 ready，交接信号匹配计划哈希后延迟 90 秒关闭已捕获路径/PID 的客户端和 CCP 启动器。完成后由 CCP --launcher 重开客户端，状态与结果持久化；无循环触发器、无执行超时，退出时移除任务定义。

