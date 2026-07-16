# Windows 安装包 MCP 二进制 staging 验收

对应规格：`spec/fix-windows-installer-mcp-staging.md`。

## 通过标准

- Windows staging 步骤复制 `claude-codex-pro.exe`、`claude-codex-pro-manager.exe` 和 `claude-codex-pro-mcp.exe`。
- NSIS 的三个 `File` 输入均能在 staging 目录中找到。
- `makensis` 能成功编译安装器。

## 验证

- 本地执行 release 构建、staging 复制及 `makensis`。
