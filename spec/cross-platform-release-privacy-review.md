# 跨平台公共发布与配置隔离审查

## 背景

发布包面向公共 Windows/macOS 用户。用户担心：

- 不同 Windows/macOS 系统下普通用户是否能正常安装和使用。
- 上传到 GitHub 的 Release 是否会把本机供应商配置、API Key、relay profile、Codex/Claude 本地配置一并带上。

## 目标

- 审查 Windows 安装包、Windows ZIP、macOS DMG、macOS ZIP 的打包输入。
- 确认 Release workflow 只打包应用二进制和必要资源，不打包用户目录配置。
- 增加自动化回归测试，防止将用户供应商配置、API Key、`memory_assist.sqlite`、Codex/Claude 本地状态打入 GitHub Release。
- 确认 Windows/macOS 公共用户安装路径使用用户本地目录或系统应用目录，不依赖开发者本机绝对路径。
- 保持诊断报告和 latest.json 不泄露密钥。

## 非目标

- 不上传真实 GitHub Release。
- 不修改用户本机现有供应商配置。
- 不重置数据库或删除用户数据。
- 不改变供应商配置导入/切换的业务能力。

## 技术约束

- 优先通过 workflow、安装脚本、结构测试证明隔离。
- 不把任何真实 API key、Bearer token、`sk-` 等秘密写入仓库。
- 只允许发布包包含构建产物、应用包、图标和必要安装脚本产物。
