# 验收：彻底移除 Multica

## 对应规格

`spec/remove-multica-completely.md`

## 通过标准

1. **残留扫描**：`git grep -il multica` 在被跟踪文件中只允许命中：本规格与验收文档、`spec/remove-multica-sidebar-features.md` 与其验收（作为历史记录保留）、仅顺带提及 Multica 的 10 份历史 spec/acceptance 与若干审计类文档（保留原样作为历史）、以及两处刻意保留的旧配置兼容测试（`settings.rs`）和 `windows_subsystem.rs` 中的“PR 工作流不含 multica”负向断言。源码、脚本、CI、安装脚本、前端、`assets/` 中除此之外命中数为 0。
2. **编译**：`cargo check --workspace` 通过，且不新增 warning 类别（允许保留移除前已有的 warning）。
3. **前端**：`npm --prefix apps/claude-codex-pro-manager run check` 与 `vite:build` 通过；`vite:build` 不再调用 `apps/codex-workflow-surface`。
4. **目录**：`apps/codex-workflow-surface`、`docs/third-party/multica`、`docs/multica-*.md` 不再存在；`crates/claude-codex-pro-core` 下无 `multica*` 文件。
5. **保留功能**：
   - `cargo test -p claude-codex-pro-core --lib codex_execution` 通过（线程 / 回合 / 流式执行行为不变；技能执行请求相关测试随功能删除）。
   - `cargo test -p claude-codex-pro-core --lib claude_desktop_computer_use` 通过。
   - `cargo test -p claude-codex-pro-manager --lib distill_pipeline` 通过。
   - `cargo test -p claude-codex-pro-manager --test windows_subsystem`：除移除前已存在的 1 条失败（`supplier_screen_matches_ccswitch_style_layout_and_drag_sorting`）外全部通过；涉及 Multica 的旧断言随功能一并删除。
6. **旧配置兼容**：有单测证明含 `multicaWorkspaceEnabled` 的旧 `settings.json` 能正常加载与更新（该键惰性保留，不报错、不影响其它设置）。
7. **发布脚本**：`node scripts/release/verify-release-workflow.js` 通过；三个 workflow 与 NSIS / DMG 脚本不再引用 workflow-surface 或 Multica 许可文件。
8. **用户数据**：`~/.claude-codex-pro/multica/` 在整个过程中未被读取、修改或删除。
9. **构建**：`cargo build --release` 成功，`target/release/claude-codex-pro.exe` 时间戳为本次构建；`--mcp-computer-use` 握手仍返回 10 个工具。

## 验证方式与证据

- 各命令输出摘要记录到最终汇报，并逐条对照上面 9 项。
- 实机确认（需用户执行）：重启 CCP 后管理工具与 Codex 侧边栏无 Multica 入口，供应商切换、蒸馏、Computer Use 正常；升级时未弹出错误。

## 不在范围内

- 对真实 Codex / Claude 客户端的端到端回归（本机会话无法自动执行）。
- 用户本机 `~/.claude-codex-pro/multica/` 的清理。
