# 修复 Windows 安装包 MCP 二进制缺失

## 背景

Windows CI 的 NSIS 安装器声明了 `claude-codex-pro-mcp.exe`，但 staging 步骤未复制该二进制，导致构建失败。

## 目标

CI 在执行 NSIS 前将启动器、管理器和 MCP 三个 release 二进制全部放入 `dist/windows/app`。

## 非目标

不调整安装器行为、文件名、发布流程或任何应用功能。

## 验收依据

见 `acceptance/fix-windows-installer-mcp-staging.md`。
