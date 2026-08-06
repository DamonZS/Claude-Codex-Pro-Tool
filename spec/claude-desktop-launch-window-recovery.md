# Windows Claude Desktop 启动窗口恢复

## 背景

用户在 Windows 管理工具中点击“启动/重启 Claude”后，界面提示“Claude Desktop 已启动。未检测到 Claude 进程，启动请求没有形成可用窗口”。当前启动链路可能只确认某个可执行文件或 `shell:AppsFolder` 启动命令已被系统接受；命令成功退出不等于 Claude 进程和可见窗口已经形成。尤其对 Microsoft Store / MSIX 安装，直接可执行文件路径、Start Menu 条目和 AppsFolder 激活入口具有不同语义，单次请求被接受后仍可能没有实际启动应用。

现有 `spec/claude-launch-inspector-readiness.md` 规定了启动后的真实探测和失败反馈，`spec/stale-app-path-self-healing.md` 规定了失效路径回退，但均未规定“某一启动入口返回成功、实际却未形成进程/窗口”时继续尝试下一入口。因此本规格只补齐 Windows Claude Desktop 的启动恢复闭环。

## 目标

本次包含：

- Windows 下点击“启动/重启 Claude”后，形成真实 Claude Desktop 进程和可见主窗口。
- 将“启动命令已接受”与“Claude 已可用”分开判断。
- 某一安全启动入口未形成进程时，按确定顺序尝试其他已发现入口。
- 所有入口均失败时返回可操作、脱敏的中文诊断。
- 为启动入口选择、回退和就绪判定增加可重复的回归测试。

本次不包含：

- 不修改 Claude Desktop 官方安装文件、ASAR、签名、Electron Fuse 或账号状态。
- 不修改供应商、路由、本地代理、模型、API Key 或 Claude 配置。
- 不重新引入 Node Inspector / CDP 启动参数。
- 不改变 macOS 既有启动行为，除非为保持跨平台编译所需的最小调整。

## 用户视角描述

Claude 未运行时，用户点击“启动/重启 Claude”，管理工具应打开真实 Claude Desktop 窗口。Claude 已运行时再次点击，管理工具应结束旧 Claude 进程，等待其退出，再启动一个可见的新窗口。只有真实进程和窗口已经观察到后，界面才提示启动成功；否则应说明尝试了哪些类型的启动入口以及失败阶段，而不是把系统接受启动请求描述为“已启动”。

## 功能要求

- Windows 启动链路必须区分以下状态：启动入口调用失败、启动请求已接受但未观察到进程、进程已出现但尚无可见窗口、进程与可见窗口均已就绪。
- 非 MSIX 的有效 `Claude.exe` 可以直接启动；WindowsApps / MSIX 安装必须使用系统支持的包激活入口，不得把受保护的 WindowsApps 可执行文件当作普通桌面程序直接运行。
- 启动入口返回成功后必须在有界时间内轮询真实 Claude 进程及其可见顶层窗口。仅命令退出码为零不能返回成功。
- 如果当前入口在短时探测后仍没有产生 Claude 进程，启动链路必须继续尝试下一种已发现且安全的入口。入口顺序必须确定、去重并可测试。
- 一旦观察到 Claude 进程，不得为了尝试其他入口再重复启动；应继续等待窗口、尝试激活既有窗口，并在总超时后返回“进程存在但窗口未就绪”的明确结果。
- `status=ok` 必须以本次操作后观察到 Claude 进程和属于该进程的可见窗口为依据。只有进程而没有可见窗口时不得报告普通成功。
- 重启时只允许终止已识别的 Claude Desktop 进程，必须等待旧 PID 退出后再进入同一启动与回退流程；不得扩大到 CCP、Codex 或其他 Electron 应用。
- 失败结果的 `action` 必须保持真实用户动作：首次启动为 `open`，重启为 `restart`，不得因启动失败写成无关动作。
- 诊断信息至少应包含入口类型、请求是否被系统接受、是否观察到进程、是否观察到窗口和最终失败阶段；不得记录命令中的密钥、令牌、完整环境变量或用户会话材料。
- Tauri 命令层可以附加本地模型代理状态，但代理告警不得覆盖或伪装 Claude 进程/窗口的启动结论。

## UI / 交互要求

- 保留现有“启动/重启 Claude”按钮位置、名称和提示机制。
- 操作期间沿用现有忙碌态，避免用户并发触发重复启动。
- 成功提示必须说明 Claude 窗口已打开；失败提示必须使用中文并区分“未形成进程”和“已有进程但未形成可见窗口”。
- 不在普通提示中暴露 WindowsApps 完整用户路径或其他不必要的本机隐私信息。

## 数据与接口要求

- 优先复用现有 `open_claude_desktop` Tauri command 和 `ClaudeDesktopActionResult`；如现有字段足以表达结果，不新增 IPC 接口或持久化字段。
- 路径发现继续复用现有 Claude Desktop inventory、候选路径和 AppX/MSIX 查询结果。
- 本任务不得写入或重置 Claude Desktop 配置、CCP 设置或供应商数据。
- 启动尝试日志只记录脱敏的入口类型与结果，不记录 API Key、Bearer token、账号令牌或请求正文。

## 技术约束

- 主要修改边界为 `crates/claude-codex-pro-core/src/claude_desktop.rs`；仅在命令结果合并或回归契约确有需要时修改 `apps/claude-codex-pro-manager/src-tauri/src/commands.rs` 和对应测试。
- 启动入口选择与就绪轮询应拆成可注入结果或可传入探测器的窄 helper，使“请求成功但无进程”的回退逻辑无需真实安装 Claude 即可单元测试。
- 不引入新依赖，不递归扫描磁盘，不执行未经验证的第三方脚本。
- 不使用固定的 `--inspect` / `--remote-debugging-port` 参数作为启动或就绪条件。
- 保留现有跨用户安装发现、失效路径自愈和本地代理启动行为。

## 交付范围

- Windows Claude Desktop 启动入口选择、回退、进程与窗口就绪判定修复。
- 针对首次启动、重启、入口回退、进程无窗口和全部失败的 Rust 回归测试。
- 必要的 Windows manager 命令契约测试。
- 本规格及匹配的验收文档。
- 更新后的默认 `target/release/claude-codex-pro.exe`，供用户现场验证。
