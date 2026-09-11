# 恢复注入增强的会话删除按钮验收

对应规格：`spec/codex-injected-session-delete-restore.md`

## 通过标准

1. `assets/inject/renderer-inject.js` 中 `defaultClaudeCodexProSettings()` 的 `sessionDelete` 为 `true`。
2. `claudeCodexProSettings()` 不再包含无条件覆写 `settings.sessionDelete = false;`；`enhancementsEnabled === false` 分支中的 `sessionDelete: false` 保留。
3. `attachButton()` 在 `settings.sessionDelete` 为真时创建删除按钮，写入 `dataset.codexDeleteVersion`，调用 `installActionButtonEvents` 与 `refreshActionButton`。
4. `scanLightweight()` 调用 `installDeleteButtonEventDelegation()`，且不再在扫描中移除 `.codex-delete-confirm-overlay, .codex-delete-toast`。
5. `codexActionGroupVersion` 仍为 `"6"`，`installMoreButtonEvents` 仍保留 `activateOnce` 手势去重。
6. `node --check assets/inject/renderer-inject.js` 通过。
7. `cargo test -p claude-codex-pro-core --test cdp_bridge injection_script_restores_injected_session_delete -- --nocapture` 通过。
8. `cargo test -p claude-codex-pro-core --test cdp_bridge` 全量通过（注入契约无回归）。
9. 新构建的 `target/release/claude-codex-pro.exe` 内含恢复后的注入脚本（二进制内可检索到 `group.appendChild(deleteButton)`）。

## 验证方式

- 文本契约：在 `cdp_bridge.rs` 中以 `source_between` 截取 `attachButton` 函数体，断言删除按钮创建分支存在。
- 语法：`node --check assets/inject/renderer-inject.js`。
- 构建：`cargo build --release`，随后在二进制中检索注入脚本关键片段。
- 手动：重新启动管理端后，Codex 会话行悬停出现删除按钮，点击弹出确认层。

## 证据要求

- 上述命令的真实输出（通过 / 失败计数）。
- 二进制字符串检索结果。
- 若未执行手动验证，需在交付说明中标注剩余风险。

## 非目标

- 不验证 Codex 官方客户端自身的删除实现。
- 不修改用户会话数据、`~/.codex` 配置或 CCP 用户设置。
- 不覆盖 `60d9e75` 中除注入脚本以外的改动。

## 执行记录（2026-09-11）

- `node --check assets/inject/renderer-inject.js` → 通过（Node v22.18.0，exit 0）。
- `cargo test -p claude-codex-pro-core --test cdp_bridge injection_script_restores_injected_session_delete -- --nocapture` → `1 passed; 0 failed`。
- `cargo test -p claude-codex-pro-core --test cdp_bridge` → `99 passed; 3 failed`。3 项失败与本任务无关，且在基线 `8ca18cd` 上同样失败（其断言片段在基线注入脚本中同样不存在，已用 `git show HEAD:assets/inject/renderer-inject.js` 逐条比对）：
  - `codex_multica_projects_route_is_a_read_only_codex_projection`：缺 `// The public Projects entry means Codex projects, never workspace project`。
  - `codex_multica_workspace_anchors_three_workflow_routes_after_plugin`：缺 `multicaWorkspaceStateSelectRoute(module.key);` 组合片段。
  - `injection_script_exposes_contact_tab_with_qq_groups_and_wechat_qr`：缺 `nativeProjectReadOnly`。
  因此标准 8 判定为「本次改动未引入新的注入契约失败」；基线既存失败不属于本任务范围，未做改动。
- `cargo fmt --check` → 仅剩基线既存违规 `apps/claude-codex-pro-manager/src-tauri/src/lib.rs:55`、`crates/claude-codex-pro-core/src/protocol_proxy.rs:444/461/468`；本任务触碰的 `assets/inject/renderer-inject.js` 与 `crates/claude-codex-pro-core/tests/cdp_bridge.rs` 已格式化干净。
- `cargo build --release` → 成功（`Finished release profile in 1m 30s`），`target/release/claude-codex-pro.exe`（46,259,712 字节，13:03:52）内可检索到 `group.appendChild(deleteButton);`、`forcePluginInstall: true, sessionDelete: true, markdownExport: true`、`deleteButton.dataset.codexDeleteVersion = codexDeleteVersion;`、`const codexActionGroupVersion = "6";`、`installDeleteButtonEventDelegation();`；检索不到 `settings.sessionDelete = false;` 与 `window.__codexSessionDeleteDocumentDeleteHandler = null;`。
- 换版未结束任何 CCP 进程（PID 7580 / 22760 在构建后仍存活），本地代理链路未中断。
- 未执行手动 UI 验证：当前运行实例启动于 12:45，早于注入脚本定稿时间 12:59，其内存中仍为旧脚本；需要在方便时重启管理端（或重新发起注入）后，才能观察到会话行悬停删除按钮与确认层。