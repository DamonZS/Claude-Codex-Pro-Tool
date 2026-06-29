# Codex 启动/重启按钮重新拉起修复

## 背景

用户点击管理工具顶部“启动/重启Codex”后，Codex 会被关闭，但没有重新启动。该按钮绑定 `restart_claude_codex_pro`，其后端逻辑先停止 launcher 和 Codex，再启动静默 launcher。当前 restart 路径使用通用 launcher 停止逻辑，可能保护住管理工具父级旧 launcher，导致新 launcher 命中单例锁后没有真正拉起新的 Codex。

## 目标

本次要完成：

- 让顶部“启动/重启Codex”在关闭现有 Codex 后能够重新启动 Codex。
- 复用已有前端连接修复路径中的 Codex 重启 launcher 清理策略。
- 保持按钮、Tauri command 名称和启动器二进制路径不变。

本次不包含：

- 改动 Codex 注入脚本。
- 改动 Claude 启动/重启逻辑。
- 新增用户可见配置项。
- 删除或重写 launcher 单例守卫。

## 用户视角描述

用户点击“启动/重启Codex”后，如果已有 Codex 正在运行，会先关闭旧 Codex，然后重新打开新的 Codex。如果 Codex 未运行，也应能启动 Codex。

## 功能要求

- `restart_claude_codex_pro` 必须使用 `stop_launcher_processes_for_codex_restart()` 清理旧 launcher。
- `restart_claude_codex_pro` 仍必须调用 `stop_codex_processes()` 关闭旧 Codex。
- `restart_claude_codex_pro` 仍必须调用 `spawn_claude_codex_pro_launch(...)` 启动静默 launcher。
- 前端顶部按钮仍调用 `actions.restartCodex()`。

## 数据与接口要求

- 不新增 Tauri command。
- 不改变 `LaunchRequest` 结构。
- 不改变 `launch_claude_codex_pro` 行为。

## 技术约束

- 优先修改 `apps/claude-codex-pro-manager/src-tauri/src/commands.rs`。
- 回归测试放在 `apps/claude-codex-pro-manager/src-tauri/tests/windows_subsystem.rs`。
- 不终止 Codex 进程做验证；只做构建和回归测试。

## 交付范围

- Codex restart command 修复。
- UI/command 回归测试。
- 本规格文档与对应验收标准。
