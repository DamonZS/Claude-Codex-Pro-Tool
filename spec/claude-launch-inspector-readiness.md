# Claude 启动后调试端口就绪反馈

## 背景

概览页的“启动/重启Claude”会关闭已有 Claude Desktop 并重新启动，但当前命令只确认启动请求已发出。现场验证显示 Claude 进程命令行包含 `--inspect`，但 `127.0.0.1:9229/json/version` 没有响应，修复前端连接因此无法通过 Node Inspector 注入。

Electron 官方文档说明 `--inspect` 应让主进程监听 V8 Inspector 端口；同时 Electron Fuses 的 `nodeCliInspect` 可以让应用忽略 `--inspect`、`--inspect-brk` 等参数。Claude 官方 MSIX 版本可能禁用该能力，因此本工具不能把命令行参数等同于可注入端口。

## 目标

- 启动/重启 Claude 后，管理工具必须短时轮询真实 Claude 进程和调试端口状态。
- 如果 Node Inspector 或页面 CDP 端口已验证可用，返回成功并说明可继续修复/注入。
- 如果 Claude 已启动但调试端口未就绪，返回可见告警和中文原因，不再显示为普通成功。
- 修复入口继续只在真实端口可用时尝试注入。

## 非目标

- 不修改 Claude Desktop 官方安装文件。
- 不绕过 Claude 官方 MSIX、签名、Electron Fuse 或系统安全限制。
- 不改供应商、模型列表或汉化资源写入逻辑。

## 功能要求

- `open_claude_desktop` 完成启动请求后必须检查新 Claude 进程。
- 检查必须使用现有 `detect_status_light`/`detect_debug_probe` 路径，保证与概览状态一致。
- `node_inspector_ready` 或 `debug_ports` 非空时视为调试端口可用。
- `observed_but_unverified` 时返回 `warning`，文案说明端口参数被观察到但 HTTP 端点未响应，并提示这通常意味着当前 Claude/Electron 构建不接受外部调试参数。
- 完全没有 Claude 进程时返回 `failed`。
- Tauri 命令层不得把该 `warning` 覆盖成成功；如果本地模型代理失败或未就绪，也要保留启动探测结论。

## UI / 交互要求

- 点击“启动/重启Claude”后右下角提示必须及时显示真实结果。
- 端口未就绪时提示要用中文说明：Claude 已启动，但 Inspector/CDP 未就绪，当前不能注入窗口标识。
- 概览状态仍以 `Claude 状态` 卡片为准展示运行、注入、前端/CDP 状态。

## 验收范围

- `crates/claude-codex-pro-core/src/claude_desktop.rs`
- `apps/claude-codex-pro-manager/src-tauri/src/commands.rs`
- `apps/claude-codex-pro-manager/src-tauri/tests/windows_subsystem.rs`
