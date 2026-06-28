# 跨用户 Codex 与 Claude 安装发现

## 背景

当前管理工具的启动和修复能力不能只围绕开发者本机路径工作。软件面向多个用户分发后，Codex App 与 Claude Desktop 可能来自 MSIX、独立安装器、用户目录、Program Files、便携版、macOS 系统应用目录或用户应用目录。若只识别当前机器上已有的路径，修复前端连接、启动 Codex、启动 Claude、安装维护和诊断都会在其他用户环境中失效。

## 目标

本次要完成：

- Codex 安装发现从单一路径扩展为多来源候选：运行中进程、用户保存路径、MSIX/WindowsApps、LOCALAPPDATA、APPDATA、Program Files、Program Files (x86)、ProgramW6432、macOS `/Applications` 和 `~/Applications`。
- Claude Desktop 安装发现从运行中进程和少数固定路径扩展为多来源候选。
- 启动或修复时优先使用运行中路径和用户保存路径，其次使用候选路径；找不到时给出明确诊断，不伪装成功。
- 候选路径收集必须去重，且只在存在可执行文件或可验证 bundle 时作为可启动路径。
- 为不同用户环境的目录布局增加单元测试。

本次不包含：

- 自动扫描整块磁盘。
- 自动修改用户安装目录。
- 静默信任第三方快捷方式或脚本。
- 引入新依赖。

## 用户视角描述

用户安装并打开管理工具后，即使 Codex 或 Claude 不在开发者本机的默认路径，只要位于常见安装位置，工具也应能识别并启动。若用户使用便携版或非常规目录，仍可在设置中手动选择路径，之后启动、重启、修复前端连接应复用该路径。

## 功能要求

- Codex 路径发现必须支持：
  - 用户手动保存的路径。
  - 正在运行的 `codex.exe` 路径。
  - WindowsApps 中 `OpenAI.Codex*` 与 `OpenAI.CodexBeta*`。
  - `%LOCALAPPDATA%`、`%APPDATA%` 下 OpenAI/Codex 常见目录。
  - `%LOCALAPPDATA%\Programs`、`%ProgramFiles%`、`%ProgramFiles(x86)%`、`%ProgramW6432%` 下 OpenAI/Codex 常见目录。
  - macOS `/Applications` 与 `~/Applications` 中 `Codex.app`、`OpenAI Codex.app`、`OpenAI.Codex.app`。
- Claude Desktop 路径发现必须支持：
  - 正在运行的 `Claude.exe`。
  - Windows `%LOCALAPPDATA%\Programs\Claude\Claude.exe`。
  - Windows `%LOCALAPPDATA%\AnthropicClaude\Claude.exe`。
  - Windows `%APPDATA%`、`%ProgramFiles%`、`%ProgramFiles(x86)%`、`%ProgramW6432%` 下常见 Claude Desktop 目录。
  - Windows AppX/MSIX 查询结果。
  - macOS `/Applications/Claude.app` 与 `~/Applications/Claude.app`。
- 启动 Claude 时，如果没有运行中路径，也必须尝试候选可执行文件，然后再回退 Start menu / shell AppsFolder。
- 修复前端连接自动终止或重启 Codex 时，必须使用增强后的 Codex 路径发现。

## UI / 交互要求

- 现有设置页和概览页不需要大改。
- 找不到路径时，错误文案应说明会尝试多来源发现，并提示用户手动选择路径。
- 不新增复杂配置项。

## 数据与接口要求

- 不新增数据库。
- 不写入用户配置，除非用户已有手动保存路径流程触发。
- 不记录敏感环境变量值，只记录必要路径诊断。

## 技术约束

- 沿用 Rust core 中现有 `app_paths` 与 `claude_desktop` 模块。
- 不引入新依赖。
- 不递归扫描大目录。
- 保持路径候选函数可单元测试。

## 交付范围

- `crates/claude-codex-pro-core/src/app_paths.rs`
- `crates/claude-codex-pro-core/src/claude_desktop.rs`
- `crates/claude-codex-pro-core/tests/launcher.rs`
- 必要的内联单元测试
- 本规格文档
- 对应验收标准
