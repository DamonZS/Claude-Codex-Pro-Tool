# Claude 会话管理真实数据链路

## 背景

会话管理页已经有“Claude 会话管理”区域，但它当前是占位实现：前端固定传入空数据并显示“待接入”，后端也没有独立的 Claude 会话列表和删除命令。与此同时，Claude Code 的真实会话保存在用户目录下的 JSONL/JSON 会话源中，因此管理器显示 0 条并不代表本机没有 Claude 会话。

## 目标

本次工作包含：

- 使用独立 Claude 数据源读取真实 Claude 会话，不复用 Codex SQLite 或 Codex 会话状态。
- 将 `~/.claude/projects/<project>/<session>.jsonl` 作为 Claude Code 主会话源。
- 兼容 `~/.claude/sessions`、`claude-code-sessions`、`local-agent-mode-sessions` 及可解析的 Claude JSON/JSONL 会话文件。
- 兼容 `~/.claude/audit.jsonl`、`~/.claude/local.json` 和 `~/.claude/local_*.json(l)` 直接会话源。
- 在管理器中支持刷新、按项目分组、显示标题/更新时间/模型/来源数量。
- 点击会话条目后按需读取并显示真实会话上下文浮框。
- 支持删除单个 Claude 会话；删除前必须生成可恢复的本地备份，备份失败时不得删除源文件。
- 对畸形行或无法解析的候选文件进行隔离处理：有效会话仍可返回，并向命令结果报告部分读取失败。

本次不包含：

- 不读取 Claude 官方网页 LevelDB、IndexedDB、Cookie 或二进制缓存。
- 不把完整对话正文返回前端或写入普通日志。
- 不修改盘古记忆数据库、Claude 中文注入或 Codex 会话数据库。
- 不提供会话正文编辑、批量删除或跨应用迁移。
- 不在会话列表首次加载时读取或返回完整会话正文。

## 用户视角描述

用户打开“会话管理”后，Claude 区域自动读取本机真实 Claude 会话。用户可点击“刷新”重新扫描，也可在项目分组中删除某条会话。删除成功后，该会话从列表消失，管理器提示备份位置；如果备份或删除失败，原会话保持不变并显示明确错误。

## 功能要求

### 会话发现

- 主路径只扫描 `~/.claude/projects` 下每个项目目录的顶层 `.jsonl` 会话文件；`subagents` 等嵌套执行记录不得作为独立用户会话重复展示。
- 补充路径可扫描 `.claude/sessions`、`.claude/claude-code-sessions`、`.claude/local-agent-mode-sessions`、`.config/claude-code-sessions` 和 `.config/local-agent-mode-sessions` 中的 `.json` / `.jsonl`。
- 文件结果按规范化源路径和会话 ID 幂等去重。
- 扫描必须在 Tauri blocking 线程池中运行，避免大文件阻塞界面。

### 会话解析

- JSONL 必须逐行流式读取，不允许为了取列表元数据一次性加载整个大文件。
- 至少识别 `user`、`assistant`、`custom-title`、`ai-title` 等 Claude Code 记录。
- 会话 ID 优先使用记录中的 `sessionId`，缺失时仅对单会话文件回退到文件名。
- 标题优先级：用户自定义标题、AI 标题、第一条可读用户输入、文件名。
- 项目路径优先使用记录中的 `cwd`；缺失时使用会话文件父目录的项目标识。
- 更新时间优先使用最后一条有效记录时间；无法解析时使用文件修改时间。
- 前端只接收列表元数据，不接收完整消息正文、工具参数、token、cookie 或密钥。

### 删除与备份

- 删除请求必须包含会话 ID 和扫描返回的源路径。
- 后端必须重新发现并验证源路径位于允许的 Claude 会话根目录中，禁止按任意前端路径删除文件。
- 删除前将完整源文件备份到 `~/.claude-codex-pro/backups/claude-sessions/`。
- 备份文件名必须稳定可审查并避免覆盖既有备份。
- 备份未完成、源文件发生并发变化、目标会话不存在或源文件包含无法安全拆分的多个会话时，删除必须失败且保留原文件。

### 会话上下文查看

- 点击 Claude 会话行调用独立 `load_claude_session_context` 命令，不得把正文塞入列表接口。
- 后端必须重新发现并校验请求来源路径，只允许读取受信 Claude 会话源。
- 至少提取 user、assistant、tool、system 等可读消息文本，并保留时间和顺序。
- 首次打开默认返回最近一页，长会话通过分页加载更早内容，避免大文件序列化导致管理器再次卡死。
- 会话正文不得写入管理器普通日志；日志只记录会话 ID、分页参数和消息数量。
- 无法解析、会话不存在或来源不可信时显示明确失败状态，不显示空白浮框。

## UI / 交互要求

- 保留现有 Codex 风格项目分组视觉结构。
- Claude 面板使用独立 `claudeSessions` 状态和独立刷新/删除动作。
- 移除“待接入”和“Claude 会话扫描尚未接入”占位文案。
- 工具栏展示 Claude 会话根目录、候选源数量和真实会话数。
- 刷新与删除必须使用现有通知系统给出运行中、成功或失败反馈。
- 删除确认文案必须明确是 Claude 本地会话。
- 点击会话主区域打开上下文浮框；删除按钮点击不得同时触发浮框。
- 浮框包含标题、项目、来源类型、消息数量、可滚动消息列表、关闭按钮和“加载更早内容”状态。
- 浮框支持点击遮罩或按 Escape 关闭，并具有加载态、空态和错误态。

## 数据与接口要求

- Core 新增独立 Claude 会话模块及可测试的显式路径 API。
- Tauri 新增：
  - `list_claude_sessions`
  - `load_claude_session_context`
  - `delete_claude_session`
- 前端新增独立 `ClaudeSession` / `ClaudeSessionsResult` 类型，不把 Claude 数据伪装成 Codex 数据库结果。
- 浏览器预览桥提供 Claude 会话列表与删除 mock，以便前端开发环境可验证交互。

## 技术约束

- 不新增 npm 依赖。
- 使用现有 Rust 依赖完成流式 JSON/JSONL 解析、路径验证与备份。
- 所有路径操作使用规范化/规范路径校验，避免目录穿越。
- 普通日志只能记录会话 ID、来源类型、计数和脱敏路径，不记录标题或正文。

## 交付范围

- Core Claude 会话发现、解析、删除和测试。
- Manager Tauri 命令、注册和命令测试。
- React 独立状态、动作、页面接线、预览桥和 UI 回归测试。
- 本规格与匹配验收文档。
