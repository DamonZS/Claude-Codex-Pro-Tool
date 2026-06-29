# 设置页精简与供应商拖拽修复

## 背景

管理工具设置页目前展示了部分高级配置卡片，包括 Codex 启动参数、图片覆盖、盘古记忆和安全边界。用户在截图中明确要求删除这些设置页卡片，以减少设置页干扰。同时，供应商页的供应商卡片无法稳定拖拽移动，需要修复排序交互。

## 目标

本次要完成：

- 从设置页删除 `Codex 启动参数`、`图片覆盖`、`盘古记忆`、`安全边界`四个独立卡片。
- 保留设置页其它卡片和已有保存逻辑。
- 修复供应商页供应商卡片拖拽排序，使拖拽源在 Tauri WebView 中可稳定识别。
- 保持拖拽排序保存到现有 `relayProfiles` 顺序的逻辑不变。

本次不包含：

- 删除盘古记忆总览页、会话页或工具页的盘古记忆能力。
- 删除后端设置字段、Tauri command 或配置文件字段。
- 重写供应商管理架构。
- 修改 Claude 中文注入脚本。

## 用户视角描述

用户进入设置页后，不再看到 Codex 启动参数、图片覆盖、盘古记忆、安全边界这几个卡片。用户进入供应商页后，可以拖动供应商卡片调整顺序，松手后顺序会保存。

## 功能要求

- 设置页仍显示设置文件位置、Codex 增强矩阵、Claude 一键汉化、CLI Wrapper 和运行日志等保留内容。
- 被删除的四个卡片不得再由 `SettingsScreen` 渲染。
- 供应商卡片拖拽开始时必须写入 `dataTransfer`，避免 WebView 丢失拖拽源。
- 拖拽悬停时应预览排序，并在释放时保存最终顺序。
- 如果拖拽保存失败，仍使用现有逻辑回滚到原顺序。

## UI / 交互要求

- 供应商卡片的拖拽手柄仍使用 `GripVertical` 图标。
- 拖拽手柄应有明确的抓取光标与标题提示。
- 不新增 UI 依赖。

## 数据与接口要求

- 继续使用现有 `actions.saveSettings` 保存 `relayProfiles` 顺序。
- 不新增 Tauri command。
- 不改变设置文件 schema。

## 技术约束

- 优先修改 `apps/claude-codex-pro-manager/src/App.tsx`。
- 回归测试放在 `apps/claude-codex-pro-manager/src-tauri/tests/windows_subsystem.rs`。
- 不修改 `assets/inject/claude-chinese-inject.js`。
- 不终止 Codex 进程；如 debug manager 构建被 manager 进程锁定，只允许终止 `claude-codex-pro-manager.exe`。

## 交付范围

- 设置页 UI 渲染调整。
- 供应商拖拽事件修复。
- 回归测试更新。
- 本规格文档与对应验收标准。
