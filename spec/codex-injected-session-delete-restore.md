# 恢复注入增强的会话删除按钮

## 背景

- `60d9e75`（修复供应商编辑保存及切换问题）在取消 CCP 注入层会话删除控制时，把 Renderer 注入层的会话删除按钮一并移除：默认设置改为 `sessionDelete: false`，在 `claudeCodexProSettings()` 中无条件覆写 `settings.sessionDelete = false`，删除 `attachButton()` 中创建删除按钮的分支，并用清理逻辑替换了 `installDeleteButtonEventDelegation()`。
- 结果是：CCP 控制面「会话删除」开关（后端 `codexAppSessionDelete`，默认 `true`）与注入层行为脱节。用户在设置中保持开启，Codex 会话列表悬停也不再出现删除按钮，且没有入口可以恢复。
- `8ca18cd`（注入删除）只调整了 `installMoreButtonEvents` 的手势去重与 `codexActionGroupVersion`，未恢复按钮本体。

## 目标

- 恢复 Renderer 注入层在 Codex 会话行悬停时创建 CCP 删除按钮的能力。
- 删除按钮的开关重新由后端设置 `codexAppSessionDelete`（本地键 `sessionDelete`）驱动，默认与后端默认值 `true` 一致。
- 恢复删除按钮的点击委托与确认层流程，使按钮可用而不只是可见。
- 保留 `8ca18cd` 引入的 `installMoreButtonEvents` 手势去重逻辑与 `codexActionGroupVersion = "6"`。

## 非目标

- 不修改 Codex 原生会话数据、原生删除流程或原生存储层。
- 不修改 CCP 管理端（React / Tauri）的后端设置模型。
- 不改变导出、项目移动、Timeline 等其它会话增强的行为。
- 不改变「页面功能增强」总开关（`enhancementsEnabled === false`）的关闭语义。
- 不回滚 `60d9e75` 中 commands.rs、settings.rs、leila_deploy.rs、App.tsx 的任何改动。

## 用户视角描述

1. 用户在 CCP「客户端增强」中保持「会话删除」开启（默认开启）。
2. 用户在 Codex 会话列表某一行上悬停，行尾出现 CCP 操作组。
3. 操作组中出现垃圾桶图标按钮，悬停显示「删除」气泡。
4. 点击按钮弹出 CCP 删除确认层，确认后调用本地删除桥接并显示结果提示。
5. 若用户关闭「会话删除」，注入层不再创建删除按钮，并清除旧版本遗留的按钮与确认层。

## 功能要求

- `defaultClaudeCodexProSettings()` 中 `sessionDelete` 为 `true`。
- `claudeCodexProSettings()` 不再无条件覆写 `sessionDelete`；该值来自默认值 + 本地存储 + 后端设置 `codexAppSessionDelete` 的合并结果。
- `attachButton()` 在 `settings.sessionDelete` 为真时，向操作组追加删除按钮，写入 `dataset.codexDeleteVersion`，并安装按钮事件。
- `scanLightweight()` 重新调用 `installDeleteButtonEventDelegation()`，安装 document 级 `pointerup` / `click` 委托。
- `scanLightweight()` 不得在每次扫描时移除 `.codex-delete-confirm-overlay` 与 `.codex-delete-toast`，否则会打断用户正在使用的确认层。
- `settings.sessionDelete` 为假时，仍保留清除旧版本遗留 CCP 删除 UI 的语义：由 `attachButton()` 的 `hasUnexpectedDelete` 分支与 `removeActionGroups()` 承担。

## UI / 交互要求

- 删除按钮沿用既有样式：`actionButtonClass` + `buttonClass`，`configureSvgActionButton(button, "删除", trashIconSvg())`。
- 悬停、聚焦时显示操作气泡；指针按下阶段继续调用 `stopActionButtonEvent` 阻止事件穿透到原生行选择。
- 单次手势只打开一个确认层：`installActionButtonEvents` 的 `activateOnce` 去重逻辑保持不变。
- 确认层沿用既有 `codex-delete-confirm-overlay` 结构与明暗主题样式。

## 数据与接口要求

- 后端设置键：`codexAppSessionDelete`（Rust 侧 `codex_app_session_delete`，`default_true`）。
- 删除请求仍走既有本地桥接 `window.__codexSessionDeleteBridge`，本次不改协议。
- 不新增端点，不新增持久化字段。

## 技术约束

- 改动范围限于 `assets/inject/renderer-inject.js` 与其契约测试 `crates/claude-codex-pro-core/tests/cdp_bridge.rs`。
- 注入脚本经 `include_str!` 编译进二进制，必须重新构建 release 产物才能生效。
- 保留既有 CRLF 行尾与无 BOM 编码。

## 交付范围

- `assets/inject/renderer-inject.js`：恢复删除按钮创建、事件委托与默认设置。
- `crates/claude-codex-pro-core/tests/cdp_bridge.rs`：契约测试由「注入层不创建删除按钮」改为「注入层恢复删除按钮」。
- 本规格与对应验收文档。
- 重新构建 `target/release/claude-codex-pro.exe`。
