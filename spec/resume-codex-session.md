# 会话页「恢复会话」真实恢复（Codex）

## 背景

会话页每张卡片上的按钮文案是「恢复会话」，但点击后调用 `openSession(session)`
（`apps/claude-codex-pro-manager/src/screens.tsx`），该函数只调用
`readAitrackerSessionDetail` 读取本地正文并展示，从不启动或继续任何客户端会话；
详情页头部还挂着「蒸馏」入口，于是用户看到的是「点恢复 → 进蒸馏」。
按钮的 class 名仍是 `aitracker-session-resume`，行为却是只读查看，语义与行为不一致。

本规格只解决「真的能恢复会话」这一件事；只读查看保留，但更名。

## 目标

- 会话卡上的「恢复会话」点击后**真的把该会话恢复起来**：对 Codex 会话在新控制台
  执行 `codex resume <会话标识>`，恢复到同一个本地会话（继续对话有效）。
- 会话标识取自本机真实来源，不猜测：
  - rollout 文件名末尾的会话 UUID：`~/.codex/sessions/**/rollout-<时间戳>-<uuid>.jsonl`
    （含 `sessions.ccp-backup-*` 备份目录）；
  - `~/.codex/session_index.jsonl` 的 `id` 与 `thread_name`，用于补齐会话名。
- 恢复不可用时如实告知：卡片按钮禁用并给出原因，命令返回失败而不伪装成功。
- 只读详情入口保留为「查看会话」，与恢复入口分开。

## 非目标

- 不实现 Claude Code / WorkBuddy / DSH 会话的恢复。
- 不修改蒸馏工作台与蒸馏入口的行为。
- 不修改会话采集、标题、分页与筛选逻辑。
- 不处理网关侧 `cc12h` 计费表达式（属网关，不在本仓）。
- 不实现「在 Codex 桌面应用内定位到该会话」（桌面应用无会话定位参数）。

## 用户视角

1. 用户在会话页看到一张 Codex 会话卡片，按钮区有「恢复会话」与「查看会话」。
2. 点击「恢复会话」→ 新控制台窗口打开该会话的 Codex 交互界面，历史上下文已恢复，
   可直接继续输入。
3. 若该会话在本机没有可恢复记录（例如记录已被清理），按钮为禁用态，
   悬停提示「本机没有可恢复记录」，点击不会打开蒸馏或详情。

## 功能要求

- 数据层新增 `pub fn codex_resume_targets(home: &Path) -> BTreeMap<String, CodexResumeTarget>`：
  - 扫描 `<home>/.codex/sessions*/**/rollout-*.jsonl`，从文件名提取 UUID；
  - 读取 `<home>/.codex/session_index.jsonl`（`id`、`thread_name`）补充会话名；
  - key 使用与采集层一致的会话标识派生（`private_id("codex", "structured", uuid)`），
    保证与会话页 `sessionId` 对齐；
  - 文件名无法解析出 UUID 时跳过该文件，不报错、不伪造。
- `AgentSessionSummary` 新增 `resume_id`、`resume_name`（`#[serde(default)]`，缺省空串），
  仅 Codex 会话填充；命令层在填充标题的同一处填充。
- 新增 Tauri 命令 `resume_agent_session(sessionId: String)`：
  - 命中目标且 Codex CLI 可解析 → 以新控制台启动 `codex resume <uuid>`；
    返回 `{ ok: true, mode: "codex-cli", sessionId, resumeId, message }`；
  - 未命中或 CLI 缺失 → `{ ok: false, mode: "unavailable", message: "<原因>" }`。
- 前端：
  - 卡片按钮「恢复会话」调用 `resume_agent_session`；`resumeId` 为空时禁用并提示原因；
  - 「查看会话」保留只读详情；
  - 命令返回后按 `ok` 弹出对应 toast，不得在失败时提示成功。

## UI / 交互要求

- 按钮顺序：恢复会话（主操作）、查看会话（次操作），样式沿用现有 token，不新增依赖。
- 禁用态用 `disabled` + `title` 说明；成功/失败 toast 复用现有通知通道。
- 不新增页面、弹窗与设置项。

## 数据与接口

- `codex_resume_targets(home) -> BTreeMap<String, CodexResumeTarget { id, name }>`
- `resume_agent_session(session_id: String) -> ResumeSessionOutcome`
  `ResumeSessionOutcome { ok: bool, mode: String, session_id: String, resume_id: String, message: String }`
- 启动命令：`<codex cli> resume <uuid>`，CLI 路径复用
  `claude_codex_pro_core::app_paths::find_codex_desktop_cli()`。

## 技术约束

- 新增会话字段必须 `#[serde(default)]`，保证旧快照仍可反序列化。
- 不读取、不记录任何凭据；不修改 `~/.codex` 内的会话文件（只读）。
- 启动失败（CLI 缺失、spawn 报错）必须返回失败结果，不得吞掉。
- 不引入 npm 依赖；不改变采集与蒸馏链路。

## 交付范围

- `crates/claude-codex-pro-data/src/local_usage.rs`（resume 目标映射）与单元测试
- `crates/claude-codex-pro-data/src/aitracker.rs`（`AgentSessionSummary` 新字段）
- `apps/claude-codex-pro-manager/src-tauri/src/commands.rs`（填充 + 新命令）
- `apps/claude-codex-pro-manager/src-tauri/src/lib.rs`（命令注册）
- `apps/claude-codex-pro-manager/src/types.ts`、`src/App.tsx`、`src/screens.tsx`（按钮与调用）
- `apps/claude-codex-pro-manager/src-tauri/tests/windows_subsystem.rs`（契约测试）
- 本规格与对应验收标准