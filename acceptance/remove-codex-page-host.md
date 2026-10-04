# 验收：移除无人使用的 Codex 页面宿主执行链

## 对应规格

`spec/remove-codex-page-host.md`

## 通过标准

1. **残留扫描**：`git grep -n -i "codex_execution\|codexPageHost\|CodexPageHost\|page_host" -- crates apps assets scripts` 无命中（本规格与验收文档、历史记录除外）。
2. **编译**：`cargo check --workspace --tests` 通过，错误数为 0，不新增 warning 类别。
3. **文件**：`crates/claude-codex-pro-core/src/codex_execution.rs`、`scripts/test-workflow-page-host.cjs` 不再存在。
4. **调用图证明**：每个被删的注入函数，在删除前的脚本里都只被同一条链上的函数引用；删除后用悬空引用检查确认剩余脚本中无任何被删标识符。
5. **保留行为**：
   - `node --check assets/inject/renderer-inject.js` 通过。
   - `node --test scripts/test-titlebar-anchor.cjs` 通过。
   - `cargo test -p claude-codex-pro-core --test bridge_routes --test cdp_bridge` 通过。
   - `cargo test -p claude-codex-pro-core --lib claude_desktop_computer_use` 通过。
   - `cargo test -p claude-codex-pro-manager --lib distill_pipeline` 通过。
   - `cargo test -p claude-codex-pro-launcher --test launcher_source_contract` 通过。
   - `cargo test -p claude-codex-pro-manager --test windows_subsystem`：除移除前已存在的 `supplier_screen_matches_ccswitch_style_layout_and_drag_sorting` 外全部通过。
6. **发布接线**：`node scripts/release/verify-release-workflow.js` 通过；`npm run vite:build` 通过且不再运行 `renderer:test`。
7. **构建**：`cargo build --release` 成功，`target/release/claude-codex-pro.exe` 时间戳为本次构建；`--mcp-computer-use` 握手仍返回 10 个工具。

## 验证方式与证据

- 命令输出摘要记录到最终汇报，并逐条对照上面 7 项。
- 实机确认（需用户执行）：重启 CCP 后 Codex 增强标识、设置开关、主题、脚本、Zed Remote 入口正常。

## 不在范围内

- 对真实 Codex 客户端的端到端回归。
- 注入脚本中与本链无关的死代码（如有，单独处理）。
