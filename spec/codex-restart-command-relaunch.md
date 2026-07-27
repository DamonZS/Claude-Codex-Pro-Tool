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
- 点击顶部按钮后必须立即显示“正在重启 Codex”反馈，不能等进程重启完成后才出现提示。
- 重启完成后的反馈必须明确区分 Codex 进程与注入结果，不直接展示 launcher 内部英文状态文案。
- 进行中提示可持续展示，但成功、降级或失败等终态提示必须自动关闭，同时保留手动关闭入口。

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
- 前端在调用 `restart_claude_codex_pro` 前先提交一次可渲染的进行中提示，并等待浏览器完成绘制。
- 后端返回后，前端必须把成功结果归一化为用户可理解的中文终态；注入状态未知或未成功时不得声称“注入成功”。
- 最终提示不能继续使用 `running` 状态，否则会被误认为仍在执行而永久驻留。
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

## 快速重启与结果字段约束

- Manager 顶部按钮和客户端增强页触发的手动重启默认跳过本次重复的 Provider Sync，避免在关闭旧 Codex 后、拉起新 Codex 前进行全量供应商扫描。
- 快捷方式启动和直接运行 launcher 的正常启动流程仍保留 Provider Sync，不改变供应商同步的既有行为。
- `restart_claude_codex_pro` 返回的 payload 不得使用 `status` 或 `message` 字段覆盖 `CommandResult` 的命令状态和用户提示。
- launcher 的内部状态与消息使用 `launchStatus`、`launchMessage` 等独立字段返回，前端成功判断始终以外层 `CommandResult.status` 为准。

## Windows 保留端口兼容

- Windows 将 launcher 单例端口标记为系统保留或禁止绑定、返回 `PermissionDenied` / `os error 10013` 时，launcher 必须复用已有文件锁作为单例守卫并继续启动 Codex。
- 文件锁冲突仍表示已有 launcher 实例，不得绕过单例约束。
- 后端和 CDP 继续使用既有动态端口选择逻辑，不固定新的备用端口。
