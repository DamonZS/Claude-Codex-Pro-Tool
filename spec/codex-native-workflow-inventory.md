# Codex 原生工作流状态投影

## 背景

上游 Multica 的工作区客户端包含 Inbox、Chat Sessions 和 Automations。Codex 本机 `codex-dev.db` 已保存对应的收件箱、自动化与本地会话目录状态，但当前“我的任务”只读取线程、项目、工具、Skill 和事件，导致原生工作流状态缺失。

## 目标

- 从 Codex 本机只读数据库投影收件箱、自动化运行配置/结果和本地会话目录；本环境没有云端服务，所有数据必须来自本机真实状态。
- 在工作流页面显示这些原生状态，并明确其只读与本机来源。
- 保持上游云端 API 的语义边界：不把本地任务、活动或执行记录伪装成云端 Inbox、Usage 或 Chat Session。

## 功能要求

- 读取 `codex-dev.db` 的 `inbox_items`、`automations`、`automation_runs`、`local_thread_catalog` 表；表或列不存在时返回空集合，不得导致整个 Bootstrap 失败。
- 所有投影项包含 `source: "codex_native"`，正文仅保留受限摘要，不复制完整消息或密钥。
- 自动化项按 `automation_id` 关联最多 500 条 `automation_runs` 摘要；没有对应自动化项的运行记录不伪造自动化实体。
- 会话目录项同时提供 `id` 与 `thread_id`，两者必须相等；跨 `local_thread_catalog` 与 `threads` 来源按真实 thread ID 去重。
- 支持 Bootstrap 与分页 Query 的一致资源键：`codex_native_inbox`、`codex_native_automations`、`codex_native_chat_sessions`。
- 前端在“我的任务”中展示数量、状态和来源；空态与不可用态可见。

## 非目标

- 不实现上游云端 Inbox/Chat API 的写入、通知推送或跨工作区同步。
- 不把本机状态伪装成上游服务端统计、计费或云端会话。

## 交付范围

- `multica_workspace.rs` 原生只读投影与资源键。
- `renderer-inject.js` 原生工作流状态区域。
- Rust 回归测试、规格与验收文档。
