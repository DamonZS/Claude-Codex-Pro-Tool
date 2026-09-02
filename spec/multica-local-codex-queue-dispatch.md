# 本地 Codex 队列领取与派发

## 背景

“我的任务”在 Agent 分配后会创建本地 `queued` 执行绑定，但原有实现要求界面再次调用显式创建接口并提供 prompt，因而无法自动生成并绑定当前 Codex 原生会话。

上游 Multica 的 `ClaimTaskByRuntime`、`StartTask` 明确区分排队、领取派发和运行。本地只接入当前已打开 Codex 页面 host，不能启动第二个 runtime 或模拟 daemon。

## 目标

- 为已分配 Agent 的 `queued` 绑定提供受控领取并派发接口。
- 从本地 Issue 的标题、描述和 Agent 指令构造明确、可追溯的原生 Codex 首轮输入。
- 仅在当前 Codex 页面 host 可用、创建成功并完成 CAS 提交后，将绑定推进为 `dispatched` 并记录真实 thread handle。
- 允许“我的任务”在创建或重试分配后调用该接口；同一 binding 的重放不得创建第二个原生会话。

## 非目标

- 不实现上游 PostgreSQL/Redis 队列、daemon 轮询、任务令牌、远程 runtime 或自动状态推断。
- 不将 `dispatched` 标记为 `running`；运行仍以真实状态事件或显式启动语义推进。
- 不从 Skill、项目或 Agent 数据猜测隐藏 prompt；本切片仅使用 Issue 标题/描述和 Agent `instructions` 字段。

## 功能要求

1. 新路由接受 `bindingId`、`expectedRevision` 和调用方生成的 `leaseToken`；只接受 `BindingPending` 状态。
2. 领取前须取得本地 execution lease。若 revision、状态或 lease 冲突，拒绝且不得调用 Codex host。
3. 无当前 Codex 页面 host 时，返回 `codex_page_host_unavailable`，释放本次获得的 lease，binding 保持 `queued`。
4. 派发前读取 binding 对应的本地 Issue 与 Agent；缺失或 assignee 已变更时拒绝并释放 lease，binding 保持 `queued`。
5. 输入格式按固定段落包含任务标题、任务描述（若存在）和 Agent 指令（若存在）；不得拼接未声明的配置或 Skill 内容。
6. 成功创建原生 thread 后，以原 binding revision 提交真实 handle，状态变为 `dispatched`；随后释放 lease。
7. 创建失败、提交冲突或释放失败不得报告成功。创建成功但提交失败须标记为不可自动重试的映射待处理错误，不能再次自动创建。
8. 成功重放已派发 binding 返回已持久化 handle，且不得调用 Codex host。

## 技术约束

- 使用现有 `MulticaExecutionStore` 的 reservation、lease、revision/CAS 和 `commit_execution`。
- 使用现有 `CodexExecutionService::create_thread`；不得启动 `codex.exe app-server`、CLI 或额外 runtime。
- 请求校验沿用 `routes.rs` 的闭合 JSON 模式和 ID 限制。

## 交付范围

- Core bridge 路由、最小 prompt 构造器、路由解析与定向 Rust 测试。
- 本规格和对应验收文档。
