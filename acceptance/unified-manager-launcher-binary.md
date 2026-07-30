# 验收标准：Manager 与 Launcher 单二进制统一

验证对象：`spec/unified-manager-launcher-binary.md`

## 验收项

1. 单一主二进制
   - 通过：Release 仅生成 `claude-codex-pro` 主程序，不生成 `claude-codex-pro-manager`。
   - 证据：Cargo metadata、Release 构建产物检查。

2. 双模式入口
   - 通过：无 `--launcher` 时进入 Manager；携带 `--launcher` 时在 Tauri 初始化前进入 launcher library；安装注册参数仍无界面处理。
   - 证据：入口源码契约测试、launcher library 测试。

3. 独立进程
   - 通过：Manager 使用当前可执行文件生成子进程并附加 `--launcher`；launcher bridge 打开 Manager 时运行当前程序且不附加后台参数。
   - 证据：Manager 与 launcher 源码契约测试。

4. 重启不结束 Manager
   - 通过：launcher 进程筛选排除当前 PID，Manager 重启 Codex 时只结束其他统一程序实例与 Codex。
   - 证据：watcher 单元测试和 Manager restart 契约测试。

5. watcher 参数统一
   - 通过：Windows Run key、快捷方式和立即启动命令均包含 `--launcher`。
   - 证据：watcher 单元测试。

6. Windows 单程序安装
   - 通过：NSIS 只安装一个主 exe 和 MCP；只创建一个管理入口；卸载清理旧 Manager 文件但不再安装或启动它。
   - 证据：Manager installer 契约测试和 Release 验证脚本。

7. macOS 单 App
   - 通过：DMG 只创建 `Claude Codex Pro.app`，主执行文件为 `claude-codex-pro`，App 内同时包含 MCP，不创建 Manager 第二个 App。
   - 证据：packager 契约测试、实际 DMG staging 检查。

8. Release 工作流统一
   - 通过：PR、手动 Release、自动 Release 均只 stage 和验证一个主程序；ZIP/DMG 不含旧 Manager 主程序。
   - 证据：Node 发布验证脚本和 Manager workflow 契约测试。

9. 功能回归
   - 通过：Manager 契约测试、launcher 生命周期测试、watcher 测试、TypeScript 检查和 Vite 构建通过。
   - 证据：命令输出。

10. 默认 Release 可验收
    - 通过：`cargo build --release` 成功，默认 `target/release/claude-codex-pro` 与 MCP 存在，`target/release/claude-codex-pro-manager` 不属于当前构建产物。
    - 证据：构建输出和产物检查。

