# 验收标准：macOS 启动、Claude 发现与更新恢复

验证对象：`spec/macos-runtime-recovery.md`

1. macOS App 包含统一主程序 `claude-codex-pro` 与 MCP 两个可执行文件；缺少任一项时打包失败。
2. launcher companion 能定位标准 `Contents/MacOS/claude-codex-pro`，且不保留 App Translocation 候选。
3. Claude 发现覆盖系统和用户 Applications，并校验 bundle identifier 与可执行文件。
4. Claude 启动只有在真实进程被检测到时返回成功；仅 `open` 命令成功不算完成。
5. 汉化状态、安装和恢复使用相同的 Claude bundle 解析结果及 `Contents/Resources` 根目录；目录不可写时使用 macOS 原生管理员授权，并保持原用户的 locale 与备份目录。
6. 下载失败不报告成功；保留经严格校验的 Release URL，并提供系统浏览器下载入口。
7. `cargo fmt --check`、相关 Rust 测试、Manager TypeScript 检查和前端构建通过。
8. MCP 先独立签名并立即严格验证，统一主程序只签名一次，App Bundle 最后签名并通过 `--deep --strict` 验证。
9. macOS CI 构建完成后检查 DMG 内容，并在实机验证 Codex 启动、Claude 启动及汉化；非 macOS 本机结果不得替代该项。

## 必需证据

- `installers`、`updater`、Claude Desktop、Claude 汉化和 Manager 契约测试输出。
- TypeScript 检查与 Vite 生产构建输出。
- macOS CI 的 Runner/Mach-O 架构输出、MCP/主程序/Bundle 逐项签名验证、DMG 内容及实机操作结果。

## 非验收项

- 不要求绕过 macOS 系统安全策略或在 Windows 主机生成可运行 DMG。
