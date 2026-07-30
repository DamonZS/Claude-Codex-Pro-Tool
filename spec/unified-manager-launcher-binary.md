# Manager 与 Launcher 单二进制统一

## 背景

项目当前同时发布 `claude-codex-pro` 静默 launcher 和 `claude-codex-pro-manager` 管理工具。Windows 安装器、macOS DMG、快捷方式、开发命令和 Release 工作流都需要复制并维护两套主程序，且跨平台命名与 App 结构不一致。

## 目标

- Windows 与 macOS 只发布一个主程序 `claude-codex-pro`。
- 默认启动 Manager UI；传入 `--launcher` 时运行原静默 launcher 生命周期。
- Manager 与 launcher 仍为独立进程，互不绑定窗口生命周期。
- `--register-installation` 等内部无界面命令继续由统一程序直接处理。
- 独立 MCP 程序保持不变。
- 一次性移除旧 `claude-codex-pro-manager` 构建产物、安装文件、App 和快捷方式，不保留双程序兼容阶段。

## 用户视角

用户在 Windows 双击 `claude-codex-pro.exe`，或在 macOS 打开 `Claude Codex Pro.app`，看到管理工具。管理工具启动或重启 Codex 时，会再次运行同一程序并附加 `--launcher`，后台进程保持无窗口运行。

## 功能要求

- Manager package 的唯一 bin target 名称为 `claude-codex-pro`。
- launcher package 改为内部 Rust library，不再声明独立 bin target。
- 统一入口必须在启动 Tauri 前识别 `--launcher` 与 `--register-installation`。
- `--launcher` 必须调用现有 launcher 生命周期、单例守卫、Provider Sync、helper、注入和 watchdog，不复制业务逻辑。
- Manager 生成后台进程时必须使用 `current_exe()` 并追加 `--launcher`。
- 后台 bridge 打开 Manager 时必须运行当前统一程序且不携带 `--launcher`。
- Windows watcher 的 Run key、启动快捷方式和内部启动命令必须包含 `--launcher`。
- 结束 launcher 时必须排除当前 Manager PID；不得因统一文件名结束当前管理工具。
- macOS App 内主可执行文件名统一为 `claude-codex-pro`。
- 安装与发布产物中不得出现 `claude-codex-pro-manager` 主程序。

## 打包要求

- Windows 安装目录包含 `claude-codex-pro.exe`、`claude-codex-pro-mcp.exe` 和卸载器。
- Windows 桌面与开始菜单只创建一个 `Claude Codex Pro` 管理入口。
- macOS DMG 只包含一个 `Claude Codex Pro.app` 和 Applications 链接。
- macOS App 的 `Contents/MacOS` 包含 `claude-codex-pro` 与 `claude-codex-pro-mcp`。
- CI、Release、ZIP 和验证脚本不得复制或检查旧 Manager 二进制与第二个 App。

## 技术约束

- launcher 实现保留在独立内部 crate，Manager 通过依赖调用，避免把大型实现复制进 Tauri crate。
- 不修改供应商、注入、主题、记忆、MCP 业务行为。
- 不新增第三方依赖。
- 保持 Windows `windows_subsystem` 和 macOS 无终端窗口行为。
- 保留当前工作区其他未提交改动。

## 交付范围

- Cargo workspace、launcher library、Manager 统一入口。
- Manager 后台生成与 watcher 参数。
- Windows NSIS、macOS DMG、三个 Release 工作流和发布验证脚本。
- 相关 Rust、Node 和源码契约测试。

