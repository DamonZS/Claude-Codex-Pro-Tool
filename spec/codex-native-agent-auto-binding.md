# Codex 原生智能体自动创建与任务绑定

## 背景

Multica 的原生流程是 Agent 绑定 Runtime，Issue 分配给 Agent 后创建 Task；Codex 执行器使用父会话派生子会话并记录执行映射。当前桥接只创建普通 thread，导致任务卡上的 Agent 不能形成可追踪的原生执行关系。

## 目标

- 任务执行请求携带已存在且启用的 Agent ID 时，使用当前 Codex 会话作为父会话，调用真实 `thread/fork` 创建子会话。
- 持久化 Agent、父 thread、子 thread、attempt 和幂等键，重复请求只返回同一绑定。
- Host 不支持 `subagent-v1` 或缺少父会话时返回 `unsupported`，不生成假执行状态。
- 未指定 Agent 时保留普通 `thread/start` 兼容行为。

## 非目标

- 不凭空枚举或伪造 Codex Agent；不启动第二个 Codex Runtime/CLI。
- 不改变 Multica 上游服务端的 Agent/Runtime 数据库协议。

## 数据与接口

`/multica/executions/create` 可选接收 `executionKind`（`thread`/`subagent`）、`parentThreadId` 和 `agentId`。`subagent` 必须同时提供父 thread 和 Agent ID。响应 binding 必须返回 `agentId`、`parentThreadId`、`codexThreadId`、`attemptNo` 与执行状态。

## 验收

1. 支持 `subagent-v1` 的 fake Host 收到一次 `thread/fork` 和一次 `turn/start`，并产生带 Agent/父子 thread 映射的 binding。
2. 同一幂等键重复调用不产生第二次 fork。
3. 不支持 subagent 或缺父 thread 时返回错误，store 中无 running/伪造 binding。
4. 普通 thread 创建回归测试保持通过。
