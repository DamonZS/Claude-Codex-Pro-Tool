# 移除无人使用的 Codex 页面宿主执行链

## 背景

`crates/claude-codex-pro-core/src/codex_execution.rs`（约 2100 行）实现了"通过 Codex 页面已打开的 page host 创建线程、发起回合、列出技能"的执行服务，配套有注入脚本里的 `codexPageHost*` 探测函数和 `renderer:test`。它只为 Multica 工作区服务。Multica 被彻底移除后（见 `spec/remove-multica-completely.md`），这条链上已经没有任何调用者：

- `create_thread` / `continue_thread` / `create_subagent` / `open_thread` / `list_skills` / `capabilities` 在 `codex_execution.rs` 之外没有任何调用。
- `CoreRuntimeService` 的 `codex_execution`、`codex_page_transport` 两个字段只有写入，没有读取。
- launcher 应用的 `LauncherRuntimeService::set_websocket_url` 只是在喂这个没有消费者的 transport。
- 注入脚本里 `window.__claudeCodexProCodexPageHostRequest` 没有 Rust 侧的调用方。

## 目标

本次包含：

- 删除 `codex_execution.rs` 及 `lib.rs` 中的模块声明。
- 删除 `CoreRuntimeService` 的 `codex_execution`、`codex_page_transport` 字段与 `with_codex_execution_service`、`with_codex_page_transport` 方法。
- 删除 `launcher.rs::try_inject` 里构造 page host 的代码，以及相应的源码契约断言。
- 删除 launcher 应用里 `LauncherRuntimeService` 的 page host 字段、`set_websocket_url` 及其调用。
- 删除注入脚本中全部 `codexPageHost*` / `cleanupCodexPageHostRequest` 函数、相关全局与"世代号"。
- 删除 `scripts/test-workflow-page-host.cjs`、`renderer:test` 脚本，并同步 `vite:build`、`verify-release-workflow.js`、README 中的描述。

本次不包含：

- 不改 CDP bridge（`bridge.rs`、`cdp.rs`）本身，它仍被设置、脚本、DevTools 等功能使用。
- 不改 `/settings/*`、`/user-scripts/*`、`/devtools/*` 等保留的桥接路由。
- 不改 `codexAppAssetUrl` / `loadCodexAppModule` 等若仍被其它注入功能使用的辅助函数（删除前逐个核对）。

## 用户视角

无可见变化。Codex 增强、供应商切换、主题、脚本、Zed Remote、Computer Use 等功能行为不变；注入脚本变小，启动注入时少构造一个未被使用的对象。

## 功能要求

- `cargo check --workspace --tests` 通过，不新增 warning 类别。
- 注入脚本语法检查通过；保留的 JS 测试（`test-titlebar-anchor.cjs` 等）行为不变。
- 不得误删仍被其它功能使用的注入函数：删除前需用调用图证明每个被删函数只被本链引用。

## 数据与接口要求

- 不新增、不修改任何 Tauri 命令或桥接路由。
- 不触碰用户数据。

## 技术约束

- 分支 `chore/remove-codex-page-host` 上完成，验证通过后再合并 main。
- 会话内不运行会结束真实 Claude 进程的测试。

## 交付范围

- 代码、脚本、文档清理；本规格与对应验收文档；重新构建默认 `target/release`。
