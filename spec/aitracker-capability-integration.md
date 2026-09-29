# AITRACKER 能力接入

## 背景

CCP 已有 Codex/Claude 会话和 AITRACKER 风格本地用量快照，但页面入口仍按客户端分裂，无法统一查看 AITRACKER 已接管的 Agent 工具、会话指标、工具调用和上下文蒸馏状态。

## 目标

- 以 AITRACKER `manifest.json` 为覆盖清单，在 CCP 内投影全部 36 个 Agent 工具及其平台/能力状态。
- 使用 CCP Rust 本地只读扫描结果提供统一会话摘要、详情、工具调用事件和 Agent 统计；所有 ID、路径和正文继续脱敏或按需读取。
- 提供会话分页/筛选、会话详情与转录入口，复用现有删除、恢复和上下文加载能力。
- 提供工具调用分析和 Agent 详情数据，数据仅来自已采集事件，不生成演示数值。
- 建立蒸馏任务契约，支持从选定会话生成待审批候选；本阶段先落地本地离线摘要与持久化边界，不自动上传或覆盖用户配置。

## 非目标

- 不迁移 AITRACKER 的 Next.js/Node 页面、远端服务或数据库。
- 不改变 CCP 外层导航、顶部工具栏、液态玻璃样式和供应商配置行为。
- 不在无本地证据时填充价格、延迟或成功率等虚构数据。

## 数据与接口

1. `AgentRegistry` 返回 manifest 中的工具 id、中文名、图标、颜色、平台支持、usage/context/sessions 能力和 detected 状态。
2. `AgentSessionSummary` 由本地用量事件按 `agent + session_id` 聚合，包含起止时间、事件数、Token 分项、模型/provider、状态和工具调用数。
3. `AgentToolCallEvent` 只保存工具名、Agent、时间、状态、耗时和匿名 session；不保存 prompt、工具参数或输出正文。
4. `AgentDetail` 聚合单个 Agent 的会话、Token、模型/provider 和工具排行。
5. `DistillationCandidate` 只保存选中的匿名 session 引用、脱敏摘要、状态和生成时间；生成、审批、取消必须可观察。

## UI / 交互

- 概览页继续使用现有液态玻璃外层，新增真实 Agent 数、会话数、工具调用数和数据来源标识。
- 会话页提供 Agent/状态/关键词筛选、分页、详情和上下文入口。
- Agent 详情页展示工具清单、Token/会话趋势和工具调用排行。
- 蒸馏入口展示选中会话、任务阶段、候选摘要和审批按钮；空数据明确显示“暂无本地采集记录”。

## 技术约束

- Rust + Tauri + React；采集复用 `claude-codex-pro-data::local_usage`，不引入 Node 运行时。
- 读取预算、匿名 session id、项目末段和只读约束沿用 `local-usage-aitracker-migration`。
- Tauri 命令统一返回 `CommandResult`，错误不泄露完整路径、凭证或消息正文。

## 交付范围

- 新增统一 Agent registry、事件/会话/详情/蒸馏数据契约与 Tauri 命令。
- React 增加 Agent 数据页和会话统一视图入口，保留旧客户端操作。
- 添加 Rust 单元测试、前端类型检查和 Vite 构建证据。
