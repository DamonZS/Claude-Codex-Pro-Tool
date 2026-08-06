# 验收标准：Windows Claude Desktop 启动窗口恢复

验证对象：`spec/claude-desktop-launch-window-recovery.md`

## 验收项

1. 启动请求成功不等于应用成功
   - 通过标准：自动测试构造“第一个启动入口返回成功，但探测期内没有 Claude 进程”的场景；实现继续尝试下一入口，而不是直接返回 `ok`。
   - 证据：定向 Rust 测试名称、命令和通过输出。

2. 回退入口形成进程和窗口后才成功
   - 通过标准：自动测试构造第二个入口产生 Claude 进程及所属可见窗口的场景；结果为 `status=ok`，停止后续入口，且没有重复启动。
   - 证据：测试断言启动入口调用顺序、调用次数、进程 ID 和窗口就绪状态。

3. 进程存在但窗口未就绪不伪报成功
   - 通过标准：自动测试构造已观察到 Claude 进程、总超时内始终没有可见窗口的场景；实现不再启动其他入口，最终返回 `warning` 或 `failed`，中文消息包含“进程”与“窗口未就绪”含义。
   - 证据：定向 Rust 测试输出及结果字段断言。

4. 所有入口失败时诊断完整且脱敏
   - 通过标准：自动测试覆盖入口调用失败和请求被接受但无进程两种失败；最终消息包含入口类型与失败阶段，不包含 API Key、Bearer token、账号令牌或完整环境变量值。
   - 证据：定向 Rust 测试断言与一次脱敏日志样例。

5. 重启保持正确生命周期和动作
   - 通过标准：测试证明旧 Claude PID 被限定终止并确认退出后才启动；最终 `action=restart`。旧进程无法结束时不启动新实例，且不终止 CCP、Codex 或无关 Electron 进程。
   - 证据：定向 Rust 测试或 Windows manager 契约测试通过输出。

6. Windows MSIX 与普通安装入口均受支持
   - 通过标准：测试证明普通非 WindowsApps `Claude.exe` 使用直接启动入口；WindowsApps / MSIX 候选使用包激活入口，并且入口去重、顺序确定。不得通过直接执行受保护的 WindowsApps `Claude.exe` 规避包激活。
   - 证据：启动计划/命令构造单元测试通过输出，源码检查确认未添加 Inspector/CDP 启动参数。

7. Tauri 结果不掩盖 Claude 启动结论
   - 通过标准：Claude 未形成进程/窗口时，即使本地模型代理在线也不能返回普通成功；代理失败或未就绪时可以附加告警，但不能把已经验证的 Claude 启动失败改写成成功。
   - 证据：`windows_subsystem` 命令契约测试或等价的结果合并单元测试通过输出。

8. Windows 真实首次启动可用
   - 通过标准：完全退出 Claude Desktop 后，从最新 Release 管理工具点击“启动/重启 Claude”，在约定总超时内出现新的 Claude 进程和可见主窗口；提示为成功且不再出现“启动请求没有形成可用窗口”。
   - 证据：带时间的管理工具截图、Claude 主窗口截图，以及仅包含 PID、入口类型和就绪结果的脱敏日志片段。

9. Windows 真实重启可用
   - 通过标准：Claude 已有可见窗口时再次点击按钮，旧 PID 退出，出现新的 Claude 进程和可见主窗口；按钮只触发一次重启，不留下重复 Claude 实例。
   - 证据：操作前后 PID/进程数量记录、重启后的 Claude 窗口截图和脱敏日志片段。

10. 前端、Rust 与 Release 构建通过
    - 通过标准：类型检查、前端构建、格式检查、定向测试和 Release 构建均成功，默认 Release 可执行文件时间戳为本次构建时间。
    - 证据：下列命令退出码为 0，以及 `target/release/claude-codex-pro.exe` 的路径、大小和最后写入时间。

## 必需验证

```powershell
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo fmt --check
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml claude_desktop_launch -- --nocapture
cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem claude_restart -- --nocapture
cargo build --release
Get-Item target/release/claude-codex-pro.exe | Select-Object FullName,Length,LastWriteTime
```

如果实现采用不同但同等窄范围的测试名称，可替换上述两个定向过滤器，但交付报告必须列出实际运行的测试名称与输出。Release 构建前，如项目目录内的旧 `claude-codex-pro.exe` 正在运行，必须先结束该项目构建产物进程，且不得结束项目目录外的同名程序。

## 必需证据汇总

- 修改文件列表与 `git diff --check` 结果。
- 回退入口顺序、无重复启动、进程无窗口、全部失败和重启生命周期的自动测试结果。
- 最新 Release 二进制路径、大小与时间戳。
- Windows 真实首次启动和真实重启各一次的截图、PID/进程数量与脱敏日志。
- 未修改 Claude 官方文件、供应商数据、API Key、账号状态和用户配置的确认。

## 不在范围内

- Claude 登录、订阅或远端服务可用性。
- Inspector/CDP 注入能力。
- Claude 汉化资源是否生效。
- macOS 真实启动回归；仅要求跨平台编译不因本次 Windows 修复退化。
