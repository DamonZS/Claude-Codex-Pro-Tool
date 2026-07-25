# macOS 启动、Claude 发现与更新恢复

## 背景

macOS 安装包中出现三类可复现问题：Manager 无法定位静默 Codex launcher；Claude Desktop 未被发现，导致启动与汉化失败；GitHub 安装包下载在系统代理和直连均失败时只展示底层网络错误。截图还显示 App Translocation 临时路径被加入 launcher 搜索结果，产生无效且冗长的诊断。

## 目标

- 正式 macOS Manager App 内包含 Manager、`claude-codex-pro` 和 MCP 三个运行时二进制。
- launcher 仅从当前真实 App bundle 解析 companion，并排除 App Translocation 和不存在的 debug/release 猜测路径。
- Claude Desktop 可从系统 Applications、用户 Applications、运行进程和 bundle identifier 发现。
- Claude 启动结果以真实进程为证据；未形成进程时不得报告成功。
- Claude 汉化状态、安装和恢复复用同一 bundle 解析器，并使用实际 `Contents/Resources`。
- Claude 资源不可写时使用 macOS 原生管理员授权，且提权子进程继续使用原用户的 locale、备份和诊断路径。
- 应用内更新下载失败时保持失败状态，同时提供经过仓库、版本、平台和文件名校验的系统浏览器下载入口。

## 非目标

- 不绕过 Gatekeeper、签名、公证、TCC 或 macOS 文件权限。
- 不把网络失败、仅提交启动请求或仅找到路径报告为成功。
- 不改变 Windows 现有启动、汉化和更新行为。

## 功能要求

1. macOS 打包脚本在最终签名前校验 Manager App `Contents/MacOS` 内三个运行时均存在且可执行。
2. companion 解析支持标准 bundle 布局，并拒绝 `/AppTranslocation/` 路径。
3. Claude 候选必须包含有效 `Info.plist`、受支持的 bundle identifier 和可执行的 `Contents/MacOS` 文件。
4. Claude 启动命令在有界时间内验证真实进程，失败时返回失败而非“已启动”。
5. 汉化状态与执行路径使用相同的 Claude bundle 发现结果。
6. 更新 URL 继续通过固定仓库、版本和平台资产校验；下载失败时前端显示“用系统浏览器下载”动作。
7. 诊断只展示有效、去重后的候选和简洁恢复建议。

## 验证

- Rust 测试覆盖标准 bundle、用户 Applications、App Translocation 排除、缺失 sidecar、Claude 启动验证和更新浏览器兜底。
- macOS 打包契约验证三个二进制及签名顺序。
- 前端类型检查、生产构建和 Manager 契约测试通过。
- Windows 主机验证不得替代 macOS CI/实机对 DMG、启动、权限、签名和汉化的最终验证。
