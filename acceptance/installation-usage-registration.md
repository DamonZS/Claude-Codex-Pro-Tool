# 安装设备匿名注册验收标准

对应规格：`spec/installation-usage-registration.md`

## 通过标准

1. NSIS 在安装主体完成后调用 `claude-codex-pro.exe --register-installation --app-version ${VERSION}`。
2. 注册命令不进入 Codex 启动、单实例锁或注入流程。
3. 合法主板序列号经规范化和带命名空间 SHA-256 后得到稳定的 64 位小写十六进制摘要。
4. 常见占位主板值、空值和全零值不会产生注册请求。
5. 请求体只包含 `installationId`、`appVersion`、`platform`，不包含原始主板值。
6. PowerShell 使用隐藏窗口运行，网络请求有短连接和总超时。
7. 注册失败不影响 NSIS 安装结果，且不会把硬件值写入日志。
8. launcher 与安装器现有参数、快捷方式和发布资产契约测试继续通过。
9. 修改后的默认 Release 二进制位于 `target/release`，时间戳对应本次构建。

## 验证方式

- `cargo test -p claude-codex-pro-core --test install_registration -- --nocapture`
- `cargo test -p claude-codex-pro-manager --test windows_subsystem -- --nocapture`
- `cargo test -p claude-codex-pro-launcher -- --nocapture`（若 bin 测试目标可用）
- `cargo fmt --check`
- `cargo build --release`
- 检查 `target/release/claude-codex-pro.exe`、`claude-codex-pro-manager.exe`、`claude-codex-pro-mcp.exe` 的时间戳。

## 完成证据

- 定向测试、格式检查和 Release 构建输出。
- 默认 Release 三个可执行文件的绝对路径、大小和更新时间。

## 非范围检查

- 不要求在测试中上传真实主板摘要或写入生产 D1。
- 不要求构建或发布新的 GitHub Release 安装包。
- 不验收 macOS DMG 的安装注册。
