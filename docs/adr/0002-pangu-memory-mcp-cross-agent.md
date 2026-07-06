# ADR 0002：盘古记忆的 MCP 跨 agent 共享

- 状态：已接受
- 日期：2026-07-06
- 相关规格：`spec/memory-universal-agent-brain.md`（模块 D）
- 相关验收：`acceptance/memory-universal-agent-brain.md`（阶段 4，第 17-19 项）

## 背景

盘古记忆当前只能被 Codex 侧通过 HTTP bridge（`/memory/*` 路由）读写；Claude Desktop 侧 `claude_injected` 恒为 `false`，其他 agent（Claude Code、Cursor、自定义 CLI）完全无法接入这份大脑。

阶段 1（语义检索）、阶段 2（遗忘曲线 + 分层）、阶段 3（压缩整合）已把 `memory_assist.sqlite` 打磨成一份"越用越聪明"的本地记忆库，但它仍是 Codex 独享。要让它成为**通用 agent 大脑**，必须提供一个标准协议入口，让任意支持 MCP 的 agent 接入同一份 sqlite、同一套门控与脱敏。

本 ADR 定义阶段 4（模块 D）：一个独立的 MCP server，暴露盘古记忆的读写能力。

## 决策

### 1. 传输 / 进程模型：stdio 子进程

MCP server 是一个独立的小 exe（新 crate `apps/claude-codex-pro-mcp`），通过 stdin/stdout 走 MCP stdio 传输。每个 MCP 客户端（Claude Code / Cursor / Codex CLI）按需 spawn 一个实例，用完即退，无常驻进程、无监听端口。

与现有 HTTP bridge（裸 `tokio::TcpListener`，服务 Codex 注入侧）完全解耦：两者读同一份 `memory_assist.sqlite` 和同一份设置文件，靠文件层天然共享同一份大脑，无需额外 IPC。

### 2. 库：rmcp 2.1.0 官方 SDK

用 MCP 官方 Rust SDK `rmcp`（`modelcontextprotocol/rust-sdk`），features 取 `server` + `macros` + `transport-io`（stdio）。不自己手写 JSON-RPC 协议层。workspace 是 edition 2024、已有 tokio，兼容。

### 3. 工具面：只读 search/list/recent + 写 learn

MCP server 暴露四个工具，直接映射 `MemoryAssistStore` 现有方法：

| MCP 工具 | 映射方法 | 读写 |
| --- | --- | --- |
| `memory_search` | `query` | 读 |
| `memory_list` | `list_items` | 读 |
| `memory_recent` | `list_items`（按更新时间） | 读 |
| `memory_learn` | `learn_item` | 写 |

不暴露 archive/restore/consolidate/import/export 等管理操作——那些属于管理器 UI 的职责，MCP 面只做"读大脑 + 记一条"，最小且安全。

### 4. workspace 语义：不透明字符串 + 可选参数 + 默认 global

存储层已确认 workspace 是**不透明自由字符串**（`normalize_workspace` 只做 trim / 空→global，零格式约束），现有 `codex:path:` 等键只是 Codex 侧约定前缀。因此跨 agent workspace 无需改存储层：

- MCP 工具的 `workspace` 参数**可选**，缺省用 `global`——跨 agent 默认共享同一份大脑。
- 客户端想隔离时可显式传约定键 `agent://<agent-id>/<repo>`，由一个 MCP 层的构建/解析 helper 规范化。
- 现有 `codex:` 键天然兼容（同一张表、同一套语义）。

### 5. Claude 侧接入：靠 MCP server 满足验收 #19

acceptance #19 要求"解除 `claude_injected` 恒 false，或通过 MCP 让 Claude 侧能读写"二选一。采纳 **MCP 方案**：Claude 侧（Claude Code / Desktop）通过配置这个 MCP server 即可读写盘古记忆，不改注入侧、不动 `assets/inject/claude-chinese-inject.js` 的中文注入职责（spec 明令禁止）。`claude_injected` 字段保持现状语义（表示 Claude 注入侧的记忆运行时，与 MCP 无关）。

### 6. 门控：独立开关 `memoryAssistMcpEnabled`，默认关闭 + 双层门控

新增设置字段 `memoryAssistMcpEnabled`，默认 `false`（遵循"默认不对外暴露记忆"的安全原则，与 `memoryAssistLlmSummaryEnabled` 一致的 `#[serde(default)]` 模式）。双层门控：

- **启动门控**：MCP server 进程启动时读该开关，关闭则拒绝提供服务。
- **每工具复查**：每个工具调用时复查 `memoryAssistEnabled`（总开关）；写工具 `memory_learn` 额外受总开关约束。关闭时返回明确的 MCP 错误，不静默写库。

MCP server 是独立进程，但读同一份 `SettingsStore` 文件，天然继承所有门控状态。

### 7. 一键注册 UI：盘古记忆页内注册两端

在管理器盘古记忆页加"注册 MCP"能力，一键把盘古记忆 MCP server 写入两端客户端配置：

- **Claude Desktop**：复用现有 `upsert_claude_desktop_mcp_entry(id, json_body)`，写 `mcpServers` JSON（带自动备份），对称的删除复用 `delete_claude_desktop_mcp_entry`。
- **Codex CLI**：复用现有 `upsert_context_entry_in_common_config(kind="mcp", id, toml_body)`，写 `config.toml` 的 `mcp_servers` 表。

MCP exe 的绝对路径用 `install::companion_binary_path(MCP_BINARY)` 解析（同目录兄弟二进制模式，与 `SILENT_BINARY` / `MANAGER_BINARY` 一致），新增 `MCP_BINARY = "claude-codex-pro-mcp"` 常量。注册的 server 命令即 `[<mcp-exe-绝对路径>]`（stdio，无参数）。注册入口受 `memoryAssistMcpEnabled` 开关约束——开关关闭时不提供注册。

## 后果

### 正面

- 任意支持 MCP 的 agent（Claude Code / Cursor / Codex CLI）都能接入同一份盘古记忆，真正成为通用大脑。
- stdio 子进程模型零常驻、零端口、零额外攻击面，与现有 bridge 完全解耦。
- 复用 `MemoryAssistStore` 全套方法与 `SettingsStore` 门控，MCP 层只做协议包装，不重复业务逻辑，不旁路脱敏。
- workspace 存储层零改动，`agent://` 只是约定 + helper，旧 `codex:` 键完全兼容。
- 默认关闭 + 双层门控，用户不主动开启则不对外暴露。

### 负面 / 风险

- 新增一个 crate 和 rmcp 依赖树，workspace 构建面变大。
- MCP server 是独立进程，无法直接复用 bridge 的 `BridgeContext`；门控靠各自读同一份设置文件，需保证读取逻辑一致。
- 跨 agent 真机验证（真实 MCP 客户端连接）不像单测能自动化，需要手动配置外部客户端确认。
- 一键注册 UI 复用现有 Claude Desktop / Codex 配置写入函数，改动面覆盖前端 + 命令层 + 契约测试（`windows_subsystem`），需相应更新。

## 备选方案（未采纳）

- **HTTP / SSE transport**：需要常驻进程 + 监听端口 + 鉴权，攻击面和运维成本都高于 stdio；stdio 是 MCP 本地集成的主流方式。
- **复用现有 HTTP bridge 加 MCP 端点**：bridge 是裸 TcpListener 服务 Codex 注入，混入 MCP 协议会耦合两个不同职责，且仍需常驻端口。
- **手写 JSON-RPC 协议层**：重复造轮子，rmcp 官方 SDK 已覆盖协议与 stdio 传输。
- **解除 `claude_injected` 走注入通道**：改注入侧风险高，且 spec 禁止改动 `claude-chinese-inject.js` 的中文职责；MCP 方案更干净。
- **workspace 强类型 enum**：存储层现为不透明字符串且 `codex:` 键已在用，强类型会破坏兼容且无实际收益。
