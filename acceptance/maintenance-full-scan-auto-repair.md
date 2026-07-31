# 验收标准：维护与诊断全量扫描和安全自动修复

验证对象：`spec/maintenance-full-scan-auto-repair.md`

## 验收项

1. 规格与验收文档存在
   - 通过标准：本规格与验收文档均存在并明确自动修复边界。
   - 证据：文件存在检查和 git diff。

2. macOS 能发现新版 Codex/ChatGPT bundle
   - 通过标准：路径发现测试覆盖 `ChatGPT.app`、`OpenAI ChatGPT.app` 和 bundle 内 `Contents/MacOS/ChatGPT`；普通无 Codex 特征的 ChatGPT bundle 不会被接受。
   - 证据：`claude-codex-pro-core` launcher 定向测试通过。

3. 检查按钮触发独立维护动作
   - 通过标准：`MaintenanceToolsPanel` 调用 `actions.runMaintenanceCheck()`，不再直接刷新 maintenance route。
   - 证据：manager `windows_subsystem` 源码契约测试通过。

4. 开始提示在耗时调用前显示
   - 通过标准：前端 action 先设置 running notice，再 `await waitForPaint()`，之后才调用 `run_maintenance_check`。
   - 证据：manager `windows_subsystem` 源码契约测试通过。

5. 后端执行全量扫描并持久化路径
   - 通过标准：新命令重新发现 Codex 路径，发现有效新路径后使用现有设置存储保存；同时重新获取 Claude 和 CCP 状态。
   - 证据：后端源码契约测试和相关 Rust 测试通过。

6. 只执行安全自动修复
   - 通过标准：命令可修复 CCP 自身入口、命令包装器和后端服务；不自动执行汉化、开发模式、供应商修改或用户关闭的 Watcher 启用。
   - 证据：源码审查和 manager 契约测试。

7. 完成提示和最终状态刷新
   - 通过标准：前端使用命令返回结果显示最终通知，并刷新 overview、settings、Claude Desktop 完整状态和 watcher。
   - 证据：manager `windows_subsystem` 源码契约测试和前端类型检查通过。

8. 构建与格式检查
   - 通过标准：前端 check、Vite build、Rust 格式检查和相关 Rust 测试通过；默认 `target/release` 生成最新管理工具。
   - 证据：实际命令输出和 release 可执行文件存在检查。

## 已知非目标

- 不验证或修改用户供应商凭据。
- 不自动修改官方 Codex/Claude 应用文件。
- 不要求检查动作重启任一官方应用。
