# 管理窗口关闭生命周期验收

对应规格：`spec/manager-close-process-lifecycle.md`。

- 路径过滤测试：同管理器完整路径的后台 PID 被选中；管理器自身、路径未知、不同目录同名程序、Codex 与 Claude PID 均被排除。
- Windows 路径大小写与斜杠变化仍匹配；macOS 路径保持大小写语义。
- manager 源码契约：窗口关闭和托盘退出仍触发 `app.exit(0)`，最终 `RunEvent::Exit` 调用路径限定清理。
- Windows 实际清理函数在终止前重新读取完整路径，使用单 PID 终止，不调用客户端终止或进程树终止。
- 定向 core watcher 测试和 manager 生命周期契约测试通过，保存真实命令和结果。
- Release 现场验收：窗口关闭后同 Release exe 路径的 CCP PID 消失，已有 Codex/Claude 进程保留。此项由总体任务统一执行并记录证据。

不包含：卸载开机启动、删除配置/会话、停止外部 Codex/Claude 客户端、macOS 实机测试。

## 本轮验证记录（2026-10-05）

- `cargo test -p claude-codex-pro-core --test watcher -- --nocapture`：17 项通过，退出码 0；包含完整路径隔离与 Windows 大小写/斜杠归一。
- `cargo test -p claude-codex-pro-manager --test windows_subsystem manager_exit -- --nocapture`：2 项通过，退出码 0；覆盖窗口/托盘退出回调和 Windows 终止前路径复核。
- `C:\Users\Damon\AppData\Local\Temp\ccp-close-lifecycle-20261005\VERIFICATION.txt` 保存源码契约 BASELINE/MODIFIED/ROLLBACK 的原始输出与哈希，补丁重构一致，独立副本回滚后哈希等于原始文件。
- `cargo build --release -p claude-codex-pro-manager --bin claude-codex-pro`：退出码 0，默认 `target/release/claude-codex-pro.exe` 已生成。
- Release 实机点击窗口关闭按钮：管理器 PID 28968、后台 launcher PID 1756 均退出；58972、57321、57320、57319 四个监听端口释放，原有 11 个客户端进程全部保留。`target/task-evidence/plugin-exit-20261005/close-before.json` 与 `close-after.json` 保存现场记录。
- Windows 现场关闭验收通过；macOS 实机未执行。
