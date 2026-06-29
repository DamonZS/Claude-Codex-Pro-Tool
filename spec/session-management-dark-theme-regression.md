# 会话管理列表暗色主题回归修复

## 背景

会话管理页的 Codex 会话管理区域需要使用类似 Codex 的项目分组和会话列表结构，但不应把管理工具的暗色控制台主题改成浅色背景。当前截图显示会话列表区域出现大块浅色背景，偏离管理工具整体主题。

## 目标

本次要完成：

- 保留 Codex 式项目分组、会话行、相对时间和悬停删除按钮布局。
- 将 Codex 会话列表区域恢复为管理工具暗色控制台主题。
- 用回归测试禁止 `codex-session-browser` 再使用浅色整块背景。

本次不包含：

- 修改 Codex 本地会话读取、删除、备份或修复逻辑。
- 修改盘古记忆、Claude 会话诊断或其它会话管理模块。
- 改动全局主题或其它页面背景。

## 用户视角描述

用户进入会话管理页后，Codex 会话管理区域仍按项目展示会话，但列表背景与周围管理工具卡片保持一致的暗色风格，不再出现浅色整块区域。

## 功能要求

- `codex-session-browser` 保留滚动列表和项目分组结构。
- `codex-session-main` 保留标题截断和右侧相对时间。
- 删除按钮仍仅在悬停或聚焦时显现。
- CSS 不得使用 `background: #f3eeee;` 作为会话列表背景。

## UI / 交互要求

- 会话列表容器、项目标题、会话行、悬停态、删除按钮必须使用暗色控制台变量或深色半透明背景。
- 不改变页面其它区域的背景。

## 数据与接口要求

- 不新增 Tauri command。
- 不修改会话数据结构。

## 技术约束

- 优先修改 `apps/claude-codex-pro-manager/src/styles.css`。
- 回归测试放在 `apps/claude-codex-pro-manager/src-tauri/tests/windows_subsystem.rs`。
- 不修改 `assets/inject/claude-chinese-inject.js`。

## 交付范围

- CSS 暗色主题修正。
- UI 回归测试更新。
- 本规格文档与对应验收标准。
