# Multica 本地任务租约与转录索引

## 背景

上游 Multica 的 `agent_task_queue` 以租约领取任务，并以 `task_message(task_id, seq)` 持久化按序转录。CCP 本地工作区已有 Codex 执行绑定、revision 和幂等命令，但没有等价的领取租约或可查询的任务消息索引。

## 目标

- 为一个本地执行绑定提供令牌守卫、过期可接管的短租约。
- 提供按绑定和序号幂等写入、升序读取的有界任务转录索引。
- 保持 Codex 为执行权威；本功能不创建 daemon、队列 worker、第二 Runtime 或虚构的 Codex 消息。

## 要求

- 租约包含 `lease_token`、`lease_expires_at_ms`、`last_heartbeat_at_ms`；领取、续约和释放均要求当前 revision 与令牌匹配。
- 未过期的不同令牌不得接管；过期租约可被新令牌接管。终态执行不得领取或续约。
- 消息以 `(binding_id, seq)` 唯一；相同内容重放返回既有项，不同内容冲突失败；读取始终按 `seq ASC`。
- 消息仅保存类型、工具名和最多 512 字符的摘要；不保存 prompt、完整模型输出、工具参数、命令、路径、URL、认证或密钥。
- 状态文件加载、保存和验证均必须维护上述不变量，并兼容旧状态文件（新增字段默认空）。

## 非目标

- 不复刻上游 PostgreSQL worker、远端 task queue、附件、完整 transcript 正文或 websocket 推送。
- 不从 Codex SQLite 反向写入消息；原生历史仍由只读投影负责。
