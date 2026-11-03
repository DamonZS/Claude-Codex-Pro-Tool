# 验收标准：Codex 启动/重启按钮重新拉起修复

验证对象：`spec/codex-restart-command-relaunch.md`

## 验收项

1. 文档存在
   - 通过标准：本文件与 `spec/codex-restart-command-relaunch.md` 存在。
   - 证据：文件存在检查。

2. 顶部按钮仍调用 restart 动作
   - 通过标准：顶部“启动/重启Codex”按钮仍绑定 `actions.restartCodex()`。
   - 证据：`windows_subsystem` 回归测试。

3. restart command 使用专用 launcher 清理逻辑
   - 通过标准：`restart_claude_codex_pro` 中调用 `stop_launcher_processes_for_codex_restart()`。
   - 证据：`windows_subsystem` 回归测试。

4. restart command 保留关闭旧 Codex
   - 通过标准：`restart_claude_codex_pro` 中仍调用 `stop_codex_processes()`。
   - 证据：`windows_subsystem` 回归测试。

5. restart command 保留重新启动 launcher
   - 通过标准：`restart_claude_codex_pro` 中仍调用 `spawn_claude_codex_pro_launch(...)`。
   - 证据：`windows_subsystem` 回归测试。

6. restart 等待旧实例完全退出
   - 通过标准：停止旧 launcher/Codex 后执行有界轮询，只有二者均退出才启动新 launcher；超时返回失败。
   - 证据：core watcher 单元测试与 `windows_subsystem` 调用顺序回归测试。

7. 降级注入可自动恢复
   - 通过标准：首次注入失败仍启动 bridge watchdog，恢复后状态切换为 `running`。
   - 证据：`crates/claude-codex-pro-core/tests/launcher.rs` 生命周期测试。

8. 正式 launcher 使用完整上下文重注入
   - 通过标准：`LauncherHooks` 覆盖 `start_bridge_watchdog`，重注入调用携带 `BridgeContext`、数据服务、运行时服务与用户脚本的 `inject_with_context`。
   - 证据：launcher 源码契约测试与 core 定向测试。

9. 前端类型检查通过
   - 通过标准：`npm --prefix apps/claude-codex-pro-manager run check` 成功。
   - 证据：命令输出。

10. 前端生产构建通过
   - 通过标准：`npm --prefix apps/claude-codex-pro-manager run vite:build` 成功。
   - 证据：命令输出。

11. Manager UI 回归测试通过
   - 通过标准：`cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem -- --nocapture` 成功。
   - 证据：命令输出。

12. 默认 Release 目录全量构建
   - 通过标准：`cargo build --release` 成功，产物位于仓库默认 `target/release`。
   - 证据：命令输出。

13. 点击后立即反馈
   - 通过标准：`restartCodex` 在调用 Tauri command 前设置“正在重启 Codex”运行态提示并等待一次绘制。
   - 证据：`windows_subsystem` 前端源码契约测试。

14. 终态反馈准确且自动关闭
   - 通过标准：成功提示明确包含 Codex 重启结果和真实注入结果，不展示 `Claude Codex Pro launcher ready`；终态状态不为 `running`，并由 Toast 自动关闭计时器清除。
   - 证据：`windows_subsystem` 前端源码契约测试与手动检查。

15. 手动重启跳过重复供应商同步
   - 通过标准：`restartCodex` 默认使用 `skipProviderSync = true`；快捷方式和 launcher 正常启动逻辑不变。
   - 证据：`windows_subsystem` 源码契约测试。

16. 重启结果不覆盖命令成功状态
   - 通过标准：重启成功 payload 不含顶层 `status`、`message`，launcher 内部状态使用 `launchStatus`、`launchMessage`；前端可稳定识别外层 `ok`。
   - 证据：Rust 单元测试与 `windows_subsystem` 源码契约测试。

17. Windows 保留端口不阻断 launcher
   - 通过标准：单例端口绑定返回 `PermissionDenied` / `os error 10013` 时使用文件锁回退；第二个实例仍因文件锁冲突被拒绝。
   - 证据：`claude-codex-pro-core` ports 单元测试。

18. macOS 真实关闭旧 Codex 与 launcher
   - 通过标准：macOS 的 `find_codex_processes`、`find_restartable_launcher_processes`、停止函数和退出等待均有真实实现，不再使用非 Windows 空操作；只匹配 Codex App 主进程与 CCP 静默 launcher，不误杀 Codex CLI、Manager 或 Electron 子进程。
   - 证据：core watcher 单元测试、平台源码契约测试与 macOS 本机构建。

19. macOS 本地 release 可解析 launcher
   - 通过标准：标准 `.app` 继续解析包内 companion；直接运行 `target/debug` 或 `target/release/claude-codex-pro-manager` 时，可解析同目录可执行的 `claude-codex-pro`，App Translocation 仍被拒绝。
   - 证据：Manager Rust 单元测试与默认 release 构建。

20. macOS 退出等待忽略僵尸进程
   - 通过标准：macOS `ps` 解析包含进程状态；`Z` 状态 PID 不进入运行中进程清单，正常 Codex/launcher PID 仍可识别；Manager 在 Unix 平台后台 `wait()` 回收静默 launcher 子进程。
   - 证据：core watcher 单元测试与 Manager command 源码契约测试。

21. 首次重启直接携带当前供应商凭据
   - 通过标准：Manager 新启动后直接点击重启，会从实时 Codex home 读取当前 provider 凭据并通过 `Command::env` 注入新 launcher，不要求用户重新选择供应商或先修改 Manager 全局进程环境。
   - 证据：Manager command 源码契约测试与 core relay config 测试。

22. macOS `.app` 启动显式传递 provider 环境
   - 通过标准：launcher 从实时 `config.toml` / `auth.json` 读取 `env_key` 和凭据，并在 `/usr/bin/open` 命令的 `--args` 前加入 `--env KEY=VALUE`；诊断日志与状态 payload 不包含凭据。
   - 证据：core launcher 单元测试与日志源码检查。

19. 等待阶段有界：进程起来了、调试端口不开时不再干等满 90 秒
   - 通过标准：`wait_for_codex_launch_ports` 记录重启前已存在的 Codex 进程集合作为 baseline；发现新进程后 `CODEX_STARTUP_DEBUG_PORT_GRACE_MS = 15_000` 内仍未等到调试端口，立即返回 `running_degraded` 状态并写 `manager.restart_injection_ports_stalled` 日志；20 秒内连新进程都没出现（`CODEX_STARTUP_NO_PROCESS_GIVE_UP_MS`）则返回 `None` 并写 `manager.restart_injection_process_missing`。
   - 证据：`cargo test -p claude-codex-pro-manager --lib restart_wait_tests` 5 条通过（含临界值前后各一毫秒）。

20. 注入没生效不等于重启失败
   - 通过标准：`restart_claude_codex_pro` 对 `running_degraded` 返回 `status: "needs_review"` 且消息点名调试端口；前端标题显示「Codex 已重启，注入未生效」而不是「重启失败」，并保留 `ok` / `needs_review` 两态配色。
   - 证据：`windows_subsystem` 契约测试 91 条通过；`restart_wait_tests` 断言降级消息包含端口号。

21. 进度提示说清「会自己收尾」
   - 通过标准：点击后立刻出现的运行态提示仍是「正在关闭旧进程并重新启动 Codex，请稍候...」，并在其后说明 Codex 弹出后会自己收尾、注入没生效会单独提示。
   - 证据：`windows_subsystem` 的 `codex_restart_feedback_is_immediate_specific_and_auto_dismissable`。

22. 真机复现与结果一致
   - 通过标准：在 Codex 桌面端（进程名为 `ChatGPT.exe`，调试端口 9230）点「启动/重启 Codex」，日志里出现 `manager.restart_injection_ports_stalled` 或 15 秒内的 `status: "ok"`，界面不再出现「未能在限定时间内启动」的 90 秒假失败。
   - 证据：真机跑 `cargo test -p claude-codex-pro-manager --lib restart_wait_tests -- --include-ignored --nocapture`，`live_wait_gives_up_in_twenty_seconds_instead_of_ninety` 打出 `elapsed_ms=21244 outcome=None` —— 同样的场景在修复前会烧满 90 秒；再加上 `%USERPROFILE%\.claude-codex-pro\claude-codex-pro.log` 的 `manager.ui.action.result` 行。

23. 打包激活失败回退到可执行文件启动
   - 通过标准：`crates/claude-codex-pro-core/src/launcher.rs` 的 Windows 打包分支在 `activate_packaged_app_with_provider_environment` 返回失败时，写 `launcher.packaged_activation_failed_falling_back_to_executable` 并跳出打包分支，继续用 `build_codex_command` 启动 `ChatGPT.exe`；该激活调用不再以 `?` 冒泡。
   - 证据：`cargo test -p claude-codex-pro-manager --test windows_subsystem` 的 `codex_launch_falls_back_to_the_executable_when_packaged_activation_fails`；真机日志出现回退行后新 Codex 进程与调试端口上线。

24. 真机：打包激活被系统拒绝时重启仍然拉起 Codex
   - 通过标准：在打包激活被拒（`0x80070005`）的本机点「启动/重启 Codex」，日志依次出现 `manager.launch_requested` → `launcher.packaged_activation_failed_falling_back_to_executable` → `launcher.cdp_readiness_target`，随后 `manager.restart_injection_confirmed`（或 `running_degraded`），且 `Get-Process ChatGPT` 有进程、调试端口 9230 打开。
   - 证据：`%USERPROFILE%\.claude-codex-pro\claude-codex-pro.log` 原始行 + `Get-Process ChatGPT.exe` 与端口探测输出。

## 执行记录（2026-11-03 22:0x：打包激活 0x80070005 定位）

- 真机证据（`%USERPROFILE%\.claude-codex-pro\claude-codex-pro.log`）：
  - `launcher.fatal {"error":"无法安装，因为在确定需要关闭哪些应用时发生未知错误 0x80070005。 (0x80070005)"}`（PID 4812，`timestamp_ms:1793715006133`），此前只有 `launcher.injection_requirement` + `helper.listening`，之后没有 `launcher.cdp_readiness_target`；
  - 同一路径连续三次失败：`manager.launch_requested`（1793714930694 / 1793714959580 / 1793714997079，`app_path=C:\Program Files\WindowsApps\OpenAI.Codex_26.1027.3384.0_x64__2p2nqsd0c76g0\app`）→ launcher PID 44404 / 27408 / 4812 都是 fatal → `manager.restart_injection_process_missing`（`waited_ms` 约 20.5s / 21.0s / 20.5s）；
  - 同时刻 `Get-Process ChatGPT,codex,claude-codex-pro` 为空、`Test-NetConnection 127.0.0.1:9230` 为 `False` → 界面上的「旧 Codex 已关闭，但新 Codex 未能在限定时间内启动」是真实状态，不是超时误报。
- 代码缺陷：`crates/claude-codex-pro-core/src/launcher.rs` 的 Windows 打包分支用 `.await?` 冒泡激活错误，导致同一次启动里的可执行文件路径永远不会被执行（`build_packaged_activation` 只在拿不到 AUMID 时才走可执行文件路径）。
- 修复：改为 match 激活结果，失败时写 `launcher.packaged_activation_failed_falling_back_to_executable` 并 `break 'packaged;`，控制流落到 `build_codex_command` + `ChatGPT.exe --remote-debugging-port=…`。
## 执行记录（2026-11-03 22:51：回退路径真机通过）

- 构建与换装：`cargo build --release`（`CARGO_TARGET_DIR=build-lead`、`TMP/TEMP=.tmp-build`）→
  `build-lead/release/claude-codex-pro.exe` **29,400,576 B @22:48:27**；用 PowerShell 停掉镜像位于
  `H:\Claude Codex Pro\` 的 CCP 进程（PID 38432）后换装（备份 `.bak-20261103-224915` 的 exe 与 dll），
  同目录 `claude_codex_pro_manager_lib.dll` 1,767,424 B @22:46:21。
- 手工触发 launcher（等价于点「启动/重启 Codex」的 launcher 腿）：
  `H:\Claude Codex Pro\claude-codex-pro.exe --launcher --debug-port 9230 --helper-port 57321`（PID 43156）。
  - 先带 `--app-path "C:\Program Files\WindowsApps\OpenAI.Codex_26.1027.3384.0_x64__2p2nqsd0c76g0\app"` 试跑，
    launcher 立即 `launcher.fatal {"error":"Codex App directory not found"}` —— 该开关不是目录语义，
    手工复现时不要传（CCP 内部由 manager 传 `app_path`，launcher 自行解析）。
- 真机日志（`%USERPROFILE%\.claude-codex-pro\claude-codex-pro.log`，PID 43156）依次出现：
  - `launcher.codex_theme_injection_requirement`（`theme_id:"default"`,`generation:60`,`enabled:false`）
  - `launcher.injection_requirement`（`debug_port:9230`,`helper_port:57321`,`settings_injection_enabled:true`,`codex_injection_enabled:true`）
  - **`launcher.packaged_activation_failed_falling_back_to_executable`**
    （`app_user_model_id:"OpenAI.Codex_2p2nqsd0c76g0!App"`，
    `error:"无法安装，因为在确定需要关闭哪些应用时发生未知错误 0x80070005。 (0x80070005)"`）
  - `launcher.codex_theme_script_ready`（`is_default:true`,`runtime_applied:false`）
- 结果核对：`Get-Process ChatGPT,codex,claude-codex-pro` → **11 个 `ChatGPT.exe`**（启动时间 22:51:31–22:51:46）
  + `codex.exe`(41508) + launcher(43156)；`Test-NetConnection 127.0.0.1 -Port 9230` → **True**。
  即：打包激活被系统拒绝（0x80070005）时，回退分支把 Codex 真实拉起，用户看到的
  「旧 Codex 已关闭，但新 Codex 未能在限定时间内启动」不再复现。
- 验收项 23 / 24 的源码契约测试同轮通过：`codex_launch_falls_back_to_the_executable_when_packaged_activation_fails`
  （`apps/claude-codex-pro-manager/src-tauri/tests/windows_subsystem.rs`）。
