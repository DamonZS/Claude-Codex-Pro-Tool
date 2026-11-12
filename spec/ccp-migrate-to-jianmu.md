# CCP 过渡版：更新后自动安装 Jianmu 并卸载 CCP

## 背景

CCP 已由 Jianmu（建木）取代。旧 CCP 的更新器只接受固定仓库 DamonZS/Claude-Codex-Pro-Tool 下名为 claude-codex-pro-{版本}-windows-x64-setup.exe 的安装包，且点击更新后会下载并运行该安装包。因此不能让旧 CCP 直接去下载 Jianmu，只能发布一个过渡版 CCP，让它的安装包完成迁移。

## 目标

- 发布一个新版本 CCP，旧 CCP 用户点击更新后，过渡安装包自动完成：下载 Jianmu、校验、安装、卸载 CCP。
- 迁移失败时不得破坏用户现有 CCP。

不包含：

- 不修改 update.rs 的来源、资产名与版本校验。
- 不删除用户数据、Codex 与 Claude Desktop 状态、供应商配置。
- 不处理 macOS（本轮只做 Windows x64）。

## 用户视角

用户在 CCP 关于页点击检查更新、下载并运行安装包。安装向导显示迁移进度。完成后桌面出现 Jianmu，CCP 的快捷方式、开始菜单项与卸载项消失。

## 功能要求

1. 过渡安装包不安装 CCP 程序文件，只做迁移。
2. 通过 https://toporeduce.cn/api/latest 取得 Jianmu 最新版本，读取 assets 中 jianmu-windows-x64.exe 的 url、size、sha256。
3. 下载只接受 https，下载后必须核对 sha256 与大小，缺少 sha256 或不一致即中止。
4. 将 Jianmu 安装到 %LOCALAPPDATA%\\Programs\\jianmu\\jianmu.exe；已存在则替换，被占用时先结束 jianmu.exe。
5. 仅当 Jianmu 已成功落盘且文件校验通过，才继续卸载 CCP。
6. 卸载 CCP：读取 HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\ClaudeCodexPro 的 UninstallString，以静默方式运行，并等待结束。
7. 卸载 CCP 时不删除用户数据目录与配置，只删除程序文件、快捷方式与卸载注册项。
8. 任一步失败：保留 CCP 不变，提示失败原因，并给出 Jianmu 下载页 https://github.com/DamonZS/Jianmu/releases。
9. 迁移完成后启动 Jianmu。

## UI / 交互要求

- 使用现有 NSIS MUI2 向导，增加迁移进度文案（中文与英文）。
- 失败态必须有明确文案，不静默退出。

## 数据与接口

- 输入：toporeduce.cn/api/latest 的 JSON，字段 version、assets[name].url/size/sha256。
- 输出：%LOCALAPPDATA%\\Programs\\jianmu\\jianmu.exe。

## 技术约束

- 沿用 scripts/installer/windows 下的 NSIS 流程与现有 release-assets / auto-release-installers 工作流。
- 不新增第三方依赖，下载与哈希校验使用系统自带 PowerShell。
- 安装包文件名、tag 规则保持不变，否则旧 CCP 更新器会拒绝。
- 不得把密钥或令牌写入脚本与日志。

## 交付范围

- 过渡版 NSIS 脚本改动、必要的 PowerShell 辅助脚本。
- workflow 结构测试与 NSIS 安装测试覆盖新行为。
- 本 spec 与对应 acceptance。
- 发布新版本 CCP（需用户确认后再推送 tag 与发布）。
