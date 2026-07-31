# 维护与诊断全量扫描和安全自动修复

## 背景

部分 macOS 用户在 Codex App 更新并更名为 ChatGPT 后，管理工具无法发现新的安装路径。现有“维护与诊断”页的“检查”按钮只刷新部分 Claude Desktop 状态，没有重新扫描 Codex、Claude 和管理工具，也没有开始或完成提示。路径缺失时，用户点击启动/重启 Codex 会以空 `app_path` 启动，最终报 `Codex App directory not found`。

## 目标

本次包含：

- 点击“检查”后立即显示正在全量检查的右下角提示。
- 每次检查都重新扫描 Codex、Claude Desktop 和 CCP 管理工具相关状态，不依赖旧页面缓存。
- 自动发现有效 Codex 安装路径，并保存到现有设置中供后续启动使用。
- macOS 支持新版 `ChatGPT.app`、`OpenAI ChatGPT.app` 以及 bundle 内 `ChatGPT` 主进程，同时继续兼容 Codex 旧名称。
- 对安全、明确、可重复执行的本地异常执行自动修复。
- 修复后重新加载最终状态，并在右下角提示发现路径、已修复项目和仍需人工处理的问题。

本次不包含：

- 不自动安装或恢复 Claude 本机汉化。
- 不自动开启 Claude 开发模式或修改 Claude 官方应用文件。
- 不修改供应商、API Key、模型或代理配置。
- 不在 macOS 上安装 Windows 专用 Watcher。
- 不自动重新启用用户主动关闭的 Watcher。
- 不关闭或重启 Codex、Claude；检查本身只诊断和修复本地配套设施。

## 用户视角描述

用户进入“维护与诊断”页点击“检查”，按钮立即进入忙碌状态，右下角显示“正在全量扫描并自动修复”。管理工具重新查找本机 Codex 和 Claude 安装位置，检查 CCP 入口、命令包装器和本地后端。能够安全修复的问题自动处理，完成后显示结果摘要；新发现的 Codex 路径会保存，之后“启动/重启 Codex”直接使用该路径。

## 功能要求

- 前端必须提供独立的 `runMaintenanceCheck` action，不能继续把“检查”等同于 `refreshRoute("maintenance")`。
- action 必须在发起 Tauri 调用前设置 running notice，并等待一次浏览器绘制，确保耗时操作前提示可见。
- 后端必须提供单一维护检查命令，按“扫描 -> 安全修复 -> 再扫描”的顺序执行。
- Codex 路径发现必须忽略失效的保存路径，并使用当前平台候选重新发现。
- 发现有效且与设置不同的 Codex 路径时，必须通过现有 `SettingsStore` 保存，不得另建配置文件。
- macOS 候选扫描必须支持 `/Applications`、`~/Applications` 等现有搜索根中的 Codex/ChatGPT bundle。
- `ChatGPT.app` 只有在包含可识别 Codex 特征时才可作为 Codex 候选，避免把普通消费者 ChatGPT 应用误识别为 Codex。
- macOS bundle 可执行文件应选择实际存在的 `Contents/MacOS/ChatGPT` 或 `Contents/MacOS/Codex`。
- 安全自动修复只允许覆盖 CCP 自身入口、命令包装器和本地后端服务等已有修复能力；单项失败不能阻止后续扫描和结果汇总。
- 修复完成后前端必须完整刷新 overview、settings、Claude Desktop 状态和 watcher 状态。
- 结果 payload 必须包含最终 Codex 路径、Claude 路径、已修复项目、剩余问题和可展示详情。

## UI / 交互要求

- 点击“检查”后立即显示：`正在全量扫描 Codex、Claude 和管理工具，并自动修复安全异常...`。
- 检查期间按钮禁用或显示现有忙碌状态，防止重复执行。
- 成功完成时显示包含已发现路径和修复数量的摘要。
- 存在剩余问题时使用需要注意的状态，并在消息中说明仍需人工处理的数量。
- 调用失败时沿用统一错误通知，不得静默无反应。

## 数据与接口要求

- 新增 Tauri command：`run_maintenance_check`。
- 返回结构使用现有 camelCase 序列化约定，至少包含：
  - `status`
  - `message`
  - `codexAppPath`
  - `claudeAppPaths`
  - `repairedItems`
  - `remainingIssues`
  - `details`
- 不记录 API Key、令牌或其他凭据。

## 技术约束

- 沿用 Rust + Tauri + React 现有分层和 `run` / `call` / `notifyResult` 入口。
- 路径发现逻辑放在 core，编排和设置持久化放在 manager command 层。
- 不引入新依赖。
- 单项修复失败必须被转换为结果项，不能 panic。

## 交付范围

- `crates/claude-codex-pro-core/src/app_paths.rs`
- `crates/claude-codex-pro-core/tests/launcher.rs`
- `apps/claude-codex-pro-manager/src-tauri/src/commands.rs`
- `apps/claude-codex-pro-manager/src-tauri/src/lib.rs`
- `apps/claude-codex-pro-manager/src-tauri/tests/windows_subsystem.rs`
- `apps/claude-codex-pro-manager/src/App.tsx`
- `apps/claude-codex-pro-manager/src/screens.tsx`
- `apps/claude-codex-pro-manager/src/lib/actions.ts`
- `apps/claude-codex-pro-manager/src/types.ts`
- `apps/claude-codex-pro-manager/src/tauriBridge.ts`

