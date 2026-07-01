# 应用更新后启动路径自愈

## 背景

Codex App 或 Claude Desktop 更新后，旧版本安装目录可能被系统移除或不再包含可执行文件。管理工具的概览状态、历史启动状态或前端缓存仍可能把旧路径传给后端，导致用户点击“启动/重启 Codex”或“启动/重启 Claude”后看起来没有反应。

## 目标

本次要完成：

- Codex 启动/重启时，即使 `LaunchRequest.app_path` 非空，也必须验证它是否仍指向可启动的 Codex。
- Codex 传入路径失效时，后端必须重新搜索当前可用安装路径。
- 用户点击“修复前端连接”时，必须真实重启 Codex 注入入口，并等待重启后的新前端心跳；不能因为旧注入脚本仍有新鲜心跳就直接返回成功。
- Claude 启动/重启时，如果运行中进程清单或缓存 hint 指向的 `Claude.exe` 已失效，必须跳过旧 hint 并继续候选路径发现。

本次不包含：

- 不新增 UI 配置项。
- 不修改 Codex 或 Claude 官方安装目录。
- 不为了验证而关闭用户正在运行的 Codex；但用户主动点击“修复前端连接”属于明确修复动作，可关闭并重启 Codex / `claude-codex-pro.exe`。

## 用户视角描述

用户更新 Codex 或 Claude 后，仍可在管理工具里点击启动/重启。即使管理工具上一次记录的是旧安装目录，也会自动重新发现新版本目录并启动。

当用户点击“修复前端连接”时，管理工具应关闭旧 Codex 和旧 `claude-codex-pro.exe` 启动器进程，重新启动静默启动器，并只在新的 CDP、后端和前端注入心跳可用时提示修复成功。

## 功能要求

- `normalize_launch_request` 必须校验非空 `app_path`，路径无效时回退到自动发现。
- Codex 自动发现优先级应覆盖：运行中 Codex、最新有效状态路径、用户保存路径、当前安装候选。
- 所有用于启动 Codex 的候选目录必须确认存在 `Codex.exe` / `codex.exe` 或 macOS bundle 可执行文件。
- Codex 进程识别不能只匹配 WindowsApps 安装路径，也要识别可归一化为有效 Codex 安装目录的 `codex.exe`。
- “修复前端连接”必须强制调用 Codex 重启链路，并等待 `renderer.memory_runtime` 或同等 runtime 心跳时间戳不早于本次修复开始时间。
- “修复前端连接”必须先等待 Codex 自启完成，包括 CDP `/json` 与 helper 后端上线，然后再执行前端注入确认；不得在 Codex 自启期间提前给出最终失败提示。
- 如果重启、端口上线或新心跳确认失败，修复命令必须返回失败，不得显示“已修复并确认注入”。
- Claude 启动逻辑只尝试存在的 hint；hint 不存在时继续默认候选路径和 Start menu / AppsFolder 回退。
- Claude AppX/MSIX 候选需要去重，并尽量优先较新的有效安装路径。

## UI / 交互要求

- 现有按钮和文案不新增、不删除。
- 失败时保留现有错误提示链路。

## 数据与接口要求

- 不改变 `LaunchRequest` 结构。
- 不新增 Tauri command。
- 不写入新的用户配置，除非已有保存路径流程被用户主动触发。

## 技术约束

- 沿用 Rust 后端已有路径发现模块。
- 不引入新依赖。
- 不扩大进程终止范围；验证不得杀 Codex。

## 交付范围

- `apps/claude-codex-pro-manager/src-tauri/src/commands.rs`
- `crates/claude-codex-pro-core/src/claude_desktop.rs`
- 定向回归测试
