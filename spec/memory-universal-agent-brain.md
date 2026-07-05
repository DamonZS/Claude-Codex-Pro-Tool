# 盘古记忆升级为通用 Agent 大脑

## 背景

盘古记忆当前是"以 SQLite 为存储 + Codex 会话历史为主要证据源 + Codex 前端注入为出口"的单机记忆系统，核心逻辑集中在 `crates/claude-codex-pro-core/src/memory_assist.rs`。它已经能采集用户消息、生成候选、固化长期记忆、按 workspace 隔离并注入 Codex 会话启动摘要。

但对照"让盘古记忆真正成为一个通用 agent 大脑、帮助 agent 越用越聪明"的目标，现状有四个结构性差距（已通过 GitHub 生态调研与源码调研确认）：

1. **检索只有关键词**：`score_item` / `keywords_for` 只做 CJK n-gram + ASCII token 交集打分，查"发布"找不到"push / deploy"，无语义召回、无向量、无 FTS 全文索引。
2. **压缩是全表重写**：`consolidate_items_into_lesson_manual` 每次 `DELETE FROM memory_items` 后仅写回 Top-10 bullet，原始记忆永久丢失，无分层、无增量、无 LLM 摘要。
3. **完全没有遗忘/衰减**：`access_count` 只在查询排序时给微弱加分，从不按时间/低访问自动衰减或归档，记忆只增不减。
4. **绑死 Codex、无对外协议**：`claude_injected` 硬编码为 `false`，无 MCP 层，其它 agent（Claude Code / Cursor / 自定义 CLI）无法读写这份记忆。

调研发现，成熟开源框架（mem0、Letta、graphiti、cognee、engram、YourMemory、cortexgraph、A-MEM 等）几乎全是 Python，直接嵌入会破坏本项目的 Rust + SQLite 单机架构与打包简洁性。因此本次采用**混合架构**：核心能力用 Rust 增强 `memory_assist.rs`（SQLite 原生 FTS5 + BLOB 向量，零新 C 扩展），对外用 MCP server 暴露，让整个 agent 生态都能接入这份大脑。

## 目标

本次要完成（分为四个能力模块，均在现有 `memory_assist.sqlite` 与本项目可审计代码内实现）：

### 模块 A：语义检索（FTS5 + 向量混合）

- 在 `memory_items` 之外新增 SQLite FTS5 虚表，对记忆正文做全文索引，检索时用 BM25 排序。
- 为每条长期记忆存储一个 embedding 向量（`BLOB` 列或独立表），检索时计算查询向量与记忆向量的余弦相似度。
- 采用混合检索：FTS5 全文得分 + 向量余弦得分 + 现有 `score_item` 关键词得分，用 RRF（Reciprocal Rank Fusion）或加权融合成最终排序。
- Embedding 生成走本地可用通道；当没有 embedding provider 可用时，检索必须自动退化为"FTS5 + 现有关键词打分"，不得报错、不得阻塞注入。
- 保留现有关键词打分作为兜底，确保离线/无向量环境仍可用。

### 模块 B：遗忘曲线 + 分层记忆

- 为每条记忆引入一个"强度 / 保留分"字段，基于 Ebbinghaus 遗忘曲线：随时间衰减，被检索命中（访问）时增强。
- 记忆分层：至少区分"工作/短期"与"长期/语义"两层；短期记忆若长期未被强化则自动降级或归档，不再进入注入摘要。
- 衰减与归档必须是可解释、可审计的：归档而非物理删除，保留可恢复能力，写入 `memory_events` 审计。
- 高价值记忆（高置信度、`safety-rule`、`project-rule`、被频繁命中）应豁免或减缓衰减。

### 模块 C：LLM 压缩整合（替换全表重写）

- 用"增量、分层、可恢复"的整合替换现有的 `DELETE FROM memory_items` 全表重写。
- 整合前先备份（复用现有 `create_backup`），整合过程保留原始记忆（归档而非删除），只把浓缩结果作为新的一层写入。
- 整合逻辑支持 LLM 摘要通道：当有可用 LLM 通道时，用其对同主题记忆做语义聚类 + 摘要；无通道时退化为现有规则打分抽句，但仍必须保留原始记忆、不得全表删除。
- 整合应按主题/workspace 分组增量进行，而非一次把全库折成一条。

### 模块 D：MCP 跨 agent 共享

- 提供一个 MCP server，暴露盘古记忆的读写能力（至少：search / learn / list / recent），让 Claude Code、Codex CLI、Cursor 等通过 MCP 协议接入同一份大脑。
- MCP server 复用现有 `MemoryAssistStore` 与 HTTP bridge 的鉴权/门控逻辑，不得绕过 `memoryAssistEnabled` / 写开关。
- workspace 语义需要能容纳非 Codex 来源（例如 `agent://<agent-id>/<repo>`），Codex 现有 workspace 键格式保持兼容。
- MCP server 的启用/停用由设置层门控，默认行为不得在用户未开启时对外暴露记忆。

## 用户视角描述

用户在任意支持 MCP 的 agent（Codex、Claude Code、Cursor）中工作时，都能让该 agent 读到盘古记忆里沉淀的项目规则、偏好、经验教训，并把新的经验写回同一份大脑。

用户查询"怎么发布"时，即使记忆里记的是"push 到 release 分支"，语义检索也能召回；而一年前记的、再没用过的临时结论会随遗忘曲线淡出注入摘要，不再干扰当前上下文。

用户点"提炼长期记忆"时，盘古会做增量语义整合并保留原始记忆备份，而不是把学过的东西删光只留几条 bullet。

## 功能要求

- 语义检索必须与现有 `query` / `ranked_items_for_inject_cache` 打分融合，注入摘要的排序质量只能变好、不能因为新通道不可用而变差或报错。
- Embedding 与 LLM 通道都必须有"不可用即优雅退化"路径，且退化路径要在自检（selfcheck）里可见（例如 `semantic: degraded (no embedding provider)`）。
- 遗忘衰减不得影响用户手动固化（`source=manual`）的记忆的可见性，除非用户显式归档。
- 归档的记忆必须可通过管理工具或接口列出与恢复。
- MCP server 暴露的写入能力必须复用现有脱敏（`redact_secrets` / `redact_bearer_tokens`）与相似合并逻辑，不得引入未脱敏写入路径。
- 自检（selfcheck）结果必须新增语义检索、衰减/分层、MCP 三个维度的分层状态。
- 所有新表/新列必须走兼容迁移，`SCHEMA_VERSION` 递增，旧库自动升级，不得清空 `memory_items` / `memory_candidates` / `memory_captures`。

## 数据与接口要求

- 允许在 `memory_assist.sqlite` 新增：FTS5 虚表、embedding 存储（列或表）、记忆强度/分层/归档相关列、以及必要的迁移。
- 旧数据库必须能自动迁移；迁移失败必须回退且不破坏原库。
- 后端检索接口返回值保持现有结构兼容，新增字段以增量方式追加，前端旧调用不得因此报错。
- MCP server 通过独立入口暴露，复用 `MemoryAssistStore`；鉴权/门控与现有 HTTP bridge 一致。
- Embedding / LLM provider 的配置走设置层，未配置时系统按退化路径运行。

## 技术约束

- 保持现有 Rust + Tauri + React + 注入脚本架构。
- **不引入 Python 运行时依赖**；向量与全文检索优先用 SQLite 原生能力（FTS5 已随 `rusqlite` 的 `bundled` 特性编译进来）+ Rust 侧计算。
- 若必须新增 Rust crate，优先纯 Rust、跨平台、可静态链接的库，且需在 spec 评审时说明理由；能不加则不加。
- 不修改 `assets/inject/claude-chinese-inject.js`。
- 不删除、不重置 `~/.claude-codex-pro/memory_assist.sqlite`。
- 只修改盘古记忆相关链路；不做无关 UI 重构、不动发布/打包行为。
- 所有会 spawn 外部进程的新代码，在 Windows 上必须带 `CREATE_NO_WINDOW`（沿用项目既有约定）。

## 交付范围

- 本规格文档与对应验收标准。
- 分四个模块（A 语义检索 / B 遗忘分层 / C 压缩整合 / D MCP）的实现，允许分阶段提交，A 优先。
- `memory_assist.sqlite` 的兼容迁移。
- 后端检索/整合/衰减/MCP 的 Rust 实现与单元测试。
- 自检增强与状态可见性。
- 定向测试与真实验证记录（构建、测试、必要的真机验证）。

## 分阶段计划

- **阶段 1（本次优先）**：模块 A 语义检索（FTS5 + 向量混合），含迁移、混合打分、优雅退化、自检可见性、单元测试。
- **阶段 2**：模块 B 遗忘曲线 + 分层。
- **阶段 3**：模块 C LLM 压缩整合（替换全表重写）。
- **阶段 4**：模块 D MCP 跨 agent 共享。

每个阶段独立可交付、独立验证，完成后再进入下一阶段。
