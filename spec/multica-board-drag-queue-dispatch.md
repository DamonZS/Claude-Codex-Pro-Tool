# 看板拖拽与待办队列自动派发

## 背景

“我的任务”看板需要把拖拽状态变更与本地 Codex 执行队列连接起来。任务可以在待办、进行中、审核中等工作区之间移动；上游 Agent queue worker 持续扫描队列中的 `queued` 任务并派发到可用 runtime，本地页面复用同一语义。

## 目标

- 看板任务卡可拖拽到任意状态列并通过现有 workspace upsert 持久化。
- 仅对后端已创建的真实 `queued`/`binding_pending` Agent execution binding 自动派发。
- 派发使用 binding 的 revision 和稳定 lease token，避免重复创建 Codex thread。
- Host 断连、CAS 冲突或队列不可派发时保留队列，不伪造 `running`。

## 非目标

- 不把普通 Issue 猜测为执行任务。
- 不启动第二个 Codex runtime，不修改远程云端服务。
