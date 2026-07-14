# Codex 启动/重启按钮重新拉起修复

## 背景

用户点击管理工具顶部“启动/重启Codex”后，Codex 会被关闭，但没有重新启动。该按钮绑定 `restart_claude_codex_pro`，其后端逻辑先停止 launcher 和 Codex，再启动静默 launcher。当前 restart 路径使用通用 launcher 停止逻辑，可能保护住管理工具父级旧 launcher，导致新 launcher 命中单例锁后没有真正拉起新的 Codex。

## 目标

本次要完成：

- 让顶部“启动/重启Codex”在关闭现有 Codex 后能够重新启动 Codex。
- 复用已有前端连接修复路径中的 Codex 重启 launcher 清理策略。
- 在旧 launcher 与 Codex 完全退出后再拉起新 launcher，避免新旧实例争抢单例锁和 CDP 端口。
- 首次注入未就绪或 Codex renderer 被替换后，由 launcher 自动恢复注入，无需用户反复点击重启。
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
- restart 流程必须在有界超时内轮询旧 launcher 与 Codex 是否已退出；未退出时返回失败，不得继续启动一个会命中旧单例锁的新 launcher。
- `restart_claude_codex_pro` 仍必须调用 `spawn_claude_codex_pro_launch(...)` 启动静默 launcher。
- 前端顶部按钮仍调用 `actions.restartCodex()`。
- 只要 Codex 前端注入已启用，生命周期就必须启动 bridge watchdog，包括首次注入暂未成功的降级状态。
- 正式 `claude-codex-pro` launcher 的 watchdog 必须复用当前 `BridgeContext`、数据服务、运行时服务与用户脚本包执行重注入，不能退化为缺少业务上下文的基础注入。
- watchdog 恢复成功后必须把状态从 `running_degraded` 更新为 `running`。

## 数据与接口要求

- 不新增 Tauri command。
- 不改变 `LaunchRequest` 结构。
- 不改变 `launch_claude_codex_pro` 行为。

## 技术约束

- 优先修改 `apps/claude-codex-pro-manager/src-tauri/src/commands.rs`。
- 进程退出等待逻辑放在 core watcher，manager 只负责按顺序调用停止、等待和启动。
- launcher watchdog 保留现有轻量轮询间隔，不引入新依赖或常驻 UI。
- 回归测试覆盖 `crates/claude-codex-pro-core/tests/launcher.rs`、`crates/claude-codex-pro-core/tests/watcher.rs` 与 `apps/claude-codex-pro-manager/src-tauri/tests/windows_subsystem.rs`。
- 不终止 Codex 进程做验证；只做构建和回归测试。

## 交付范围

- Codex restart command 修复。
- UI/command 回归测试。
- 本规格文档与对应验收标准。
