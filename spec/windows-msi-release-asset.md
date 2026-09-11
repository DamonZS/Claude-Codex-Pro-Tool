# Windows MSI Release 资产

## 背景

Windows Release 当前通过仓库内 NSIS 脚本生成 setup.exe 和 ZIP。用户需要一个标准 MSI 资产，以便通过 Windows Installer 或企业软件分发流程安装。

## 目标

- 在手动 `release-assets` 与自动 `auto-release-installers` 工作流中生成 Windows x64 MSI。
- 将 MSI 与现有 Windows setup.exe、ZIP 以及 macOS 资产一起上传到 Release。
- 保持默认 Tauri 配置的 bundle 开关不变；仅 Release 步骤临时启用 MSI bundle。

## 功能要求

- Windows runner 安装 WiX Toolset。
- Rust 二进制和前端完成后，使用 Tauri CLI 生成 `bundle/msi/*.msi`。
- MSI 以 `claude-codex-pro-<version>-windows-x64.msi` 命名并在产物缺失时失败。
- 自动发布的构建资产数量由 6 增加为 7。
- `latest.json` 自动收录 MSI 资产。

## 技术约束

- 复用现有 Tauri 配置、工作流、NSIS 和 ZIP 流程。
- 不改变默认 `tauri.conf.json` 的 `bundle.active`，不改变 NSIS 安装器。
- 不把 MSI 加入 ZIP 内部；MSI 作为独立 Release 资产发布。

## 交付范围

`.github/workflows/release-assets.yml`、`.github/workflows/auto-release-installers.yml`、README 的发布说明，以及对应验收文档。
