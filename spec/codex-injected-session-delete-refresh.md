# 修复注入会话删除后的列表刷新

## 背景

CCP 注入层删除会话成功后只在删除当前打开会话时刷新页面。删除其它侧栏会话时虽然立即移除了 DOM 行，但 Codex 的 React 虚拟列表会重新投影旧数据，导致已删除会话再次出现；点击该条目还可能触发 `missing source rollout`。

## 目标

- 删除成功后统一刷新 Codex 页面，让原生列表从持久化数据重新读取。
- 保留删除后的即时 DOM 移除、删除备份、撤销提示和现有删除桥接协议。

## 非目标

- 不修改 Codex 原生会话存储或删除接口。
- 不改变删除按钮、确认层、备份和撤销行为。
- 不改变当前会话识别逻辑在其它功能中的用途。

## 功能要求

1. `removeDeletedRow()` 在释放焦点并移除行后，无条件调用 `window.location.reload()`。
2. 删除成功流程调用更新后的 `removeDeletedRow(row, button)`。
3. 不保留仅针对当前会话的刷新条件。

## 技术约束

- 修改范围限于 `assets/inject/renderer-inject.js`、`crates/claude-codex-pro-core/tests/cdp_bridge.rs` 及本任务规格/验收文档。
- 注入脚本通过 `include_str!` 编译进应用，交付前必须重新构建 release 产物才能生效。

## 交付范围

- 注入脚本统一刷新删除后的 Codex 列表。
- 增加源码契约测试和对应验收记录。
