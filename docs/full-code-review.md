# 全量代码评审报告

评审时间：2026-06-22  
分支：`codex/memory-assist`  
范围：当前 staged、unstaged、untracked 文件，以及与本轮记忆辅助/插件中心/管理工具相关的核心路径。  
备份：`F:\项目代码备份\Claude-Codex-Pro-Tool-backup\docs-review-graph-source-20260622-111202`

## 结论

当前改动整体方向成立：记忆辅助走本地 SQLite、候选确认、管理工具与 Codex bridge 共用同一 store；插件中心和 Claude 包装窗口也保持了“不静默执行、不修改官方安装包”的边界。

本轮发现 8 个需要修复的问题：1 个 HIGH、5 个 MEDIUM、2 个 LOW。未发现会阻塞当前构建或测试的编译级回归。

## Findings

### HIGH

1. `crates/claude-codex-pro-core/src/memory_assist.rs:1338`

   `redact_bearer_tokens` 只匹配大小写完全为 `Bearer ` 的 token。HTTP Authorization scheme 是大小写不敏感的，用户粘贴 `authorization: bearer eyJ...`、`BEARER ...` 或 `Bearer\t...` 时，token 会原样写入 `memory_items` / `memory_candidates` / 导入包落库内容。与此同时，`source` 字段来自请求但只经过 `normalize_label`，没有经过 `redact_secrets`，所以 `source: "Bearer source.secret"` 也会被持久化。这个问题绕过了“API key、Bearer token 写入前自动脱敏”的核心安全承诺。

   建议：按 ASCII case-insensitive 方式识别 `bearer`，并允许后面跟至少一个空白字符；所有请求来源的可写字符串字段都先脱敏再归一化；增加单元测试覆盖 `bearer lower.secret`、`BEARER upper.secret`、`Bearer\twith-tab` 和 `source` 字段。

### MEDIUM

1. `crates/claude-codex-pro-core/src/routes.rs:669`

   用户关闭 `memoryAssistEnabled` 或 `memoryAssistAutoSuggestEnabled` 后，Rust 写入路径仍然可以保存记忆。`CoreRuntimeService` 的 `/memory/learn`、`/memory/candidates` 直接调用 `MemoryAssistStore`，没有读取 `BackendSettings`；管理工具的 `learn_memory_assist_item` 也同样绕过设置。复现方式：保存 `memoryAssistEnabled=false`，再调用 bridge `/memory/learn` 或 Tauri `learn_memory_assist_item`，记录仍会写入 SQLite。

   建议：bridge 的 session/search/mutating endpoints 至少按 `memoryAssistEnabled` 做硬门禁；自动候选写入还要检查 `memoryAssistAutoSuggestEnabled`。管理工具手动写入如果要允许覆盖总开关，UI 和命令参数必须显式表达“手动覆盖”，否则也应拒绝。

2. `assets/inject/renderer-inject.js:8761`

   `memoryAssistInjectEnabled` 关闭后，记忆 DOM 标识和 session loader 会停，但 `codexMemoryMaybeSuggestCandidate()` 仍然会扫描 DOM 并发送 `/memory/candidates`。用户看到标识消失，会自然理解为 DOM 注入侧记忆能力已停用；实际仍会产生本地待确认记忆，属于隐式采集副作用。

   建议：`codexMemoryMaybeSuggestCandidate()` 同时检查 `memoryAssistInjectEnabled`，或把设置拆成“隐藏标识”和“允许 DOM 自动学习”两个语义明确的开关。

3. `assets/inject/renderer-inject.js:8631`

   `codexMemoryLatestUserText()` 使用了宽泛的 `[data-testid="conversation-turn"]` 和 `main [class*="user"]`，然后直接取最后一个文本。若 Codex 页面同一个 test id 包住用户和助手 turn，最新助手回复里的“以后默认...”也会被当成用户偏好，生成待确认记忆。

   建议：自动学习路径只接受明确用户角色节点，例如 `[data-message-author-role="user"]`；如果必须支持 fallback，则先从最近 turn 推断 role，无法确认时不创建候选。

4. `crates/claude-codex-pro-core/src/memory_assist.rs:522`

   已批准的候选项还能被 `reject_candidate(id)` 改成 `rejected`。复现：创建候选，调用 `approve_candidate(id)` 写入长期记忆，再调用 `reject_candidate(id)`；第二次会成功，导致候选审核状态和长期记忆事实矛盾。

   建议：`reject_candidate` 像 `approve_candidate` 一样先读取候选并要求 `status == "pending"`，否则返回错误。

5. `crates/claude-codex-pro-core/src/memory_assist.rs:779`

   相似记忆合并会用“更长文本覆盖旧文本”，可能静默丢事实。例子：先保存 `project alpha uses npm build and cargo tests`，再保存 `project alpha uses npm build and playwright checks`；关键词 overlap 可超过阈值，最终只保留较长句，`cargo tests` 事实丢失。

   建议：V1 至少改为更严格的重复判定，例如规范化文本完全相同、包含关系、或很高阈值；如果判定为相关但不相同，应创建候选“合并/更新现有记忆”，不要静默覆盖。

### LOW

1. `crates/claude-codex-pro-core/src/memory_assist.rs:733`

   自检备份文件名只使用秒级时间戳：`memory_assist-{now_unix()}.sqlite`。一秒内连续运行两次 `run_selfcheck({ repair: true })` 会覆盖前一个备份。

   建议：文件名加入纳秒、UUID 或已生成的 backup id。

2. `apps/claude-codex-pro-manager/src/App.tsx:1461`

   手动记忆输入框在保存成功前就清空：`actions.learnMemoryAssistItem(draft); setDraft("");`。如果 Tauri 后端不可用或 SQLite 写入失败，用户输入会丢失。

   建议：让 `learnMemoryAssistItem` 返回成功/失败状态，只有成功后再 `setDraft("")`。

## 已重点复核

- `memory_assist.rs`
  - schema 初始化、CRUD、workspace + global 查询、`__all__` 管理视图。
  - 候选项 approve 已使用 transaction，避免部分写入。
  - 导入导出会在写入前重新脱敏。
  - 仍需补强大小写不敏感 Bearer 脱敏、`source` 脱敏、候选状态机和相似合并策略。
  - 自检目前主要做可打开、schema version、计数和备份，不是完整修复器；UI 文案应继续避免暗示它能修复所有损坏。
- `routes.rs`
  - 新增 `/memory/*` bridge 路由接入 `CoreRuntimeService.memory_store`。
  - bridge 日志只记录 payload key，响应日志不记录正文。
  - 仍需把 memory settings 总开关落到 Rust 侧门禁。
- `commands.rs`
  - 管理工具记忆辅助 Tauri command 使用同一 `MemoryAssistStore`。
  - 插件中心仍走 preview/install 分离。
  - Claude 中文包装窗口和官方 Claude Desktop 路径分离。
- `renderer-inject.js`
  - 记忆辅助 DOM 面板中动态文本使用 `escapeHtml`。
  - 自动学习只创建候选项，不直接写入长期记忆。
  - 当前 Codex 记忆入口的“管理工具”只打开主窗口；插件中心入口使用主窗口内跳转。
  - 仍需让 DOM 自动学习遵守 `memoryAssistInjectEnabled`，并只从明确用户 turn 取文本。
- `App.tsx`
  - 工具页包含插件中心、会话维护、记忆辅助、安装维护。
  - 设置页记忆开关使用滑块组件。
  - 手动记忆输入需要等待保存成功后再清空。

## 验证记录

本轮及上一轮已确认通过：

- `npm --prefix apps/claude-codex-pro-manager run check`
- `npm --prefix apps/claude-codex-pro-manager run vite:build`
- `node --check assets/inject/renderer-inject.js`
- `cargo test --workspace --jobs 1`
- `cargo test -p claude-codex-pro-core --test memory_assist`
- `cargo test -p claude-codex-pro-core --test bridge_routes memory_bridge_routes_learn_search_and_review_candidates`
- `cargo test -p claude-codex-pro-manager --manifest-path apps/claude-codex-pro-manager/src-tauri/Cargo.toml`
- `cargo check -p claude-codex-pro-manager --manifest-path Cargo.toml`
- `git diff --check`

## 残余风险

- `renderer-inject.js` 是高耦合 DOM 补丁脚本，Codex 官方页面 DOM 变化可能导致按钮挂载或文本提取失效；需要继续用截图/真实页面回归验证。
- Claude 中文包装窗口依赖 `claude.ai` 在 Tauri WebView 中可正常登录；如果 Claude 网页策略变化，必须在 UI 明确提示失败，而不是显示“已注入”。
- 插件中心涉及第三方资源，后续新增来源时仍必须保持“只拉元数据、安装前预览、写入前备份”的策略。
- 记忆辅助 V1 是关键词/overlap 检索，不是 embedding；召回质量有限，但符合本地离线边界。
## 修复状态

- 已修复 `crates/claude-codex-pro-core/src/memory_assist.rs` 的 Bearer/source 脱敏、相似记忆误合并、候选状态回退和自检备份覆盖问题。
- 已修复 `crates/claude-codex-pro-core/src/routes.rs` 的 memory bridge 设置门禁。
- 已修复 `assets/inject/renderer-inject.js` 的 DOM 自动学习门禁和用户 turn 采集范围。
- 已修复 `apps/claude-codex-pro-manager/src/App.tsx` 的手动记忆失败清空输入问题。
- 已修复 `apps/claude-codex-pro-manager/src-tauri/src/commands.rs` 的管理工具 memory 命令门禁。
