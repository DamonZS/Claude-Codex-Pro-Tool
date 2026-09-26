# 系统提示词部署控制台与实时日志验收

对应规格：`spec/prompt-deployment-shell-logs.md`。

## 必须通过

- [x] 页面移除顶部当前提示词状态/启用方式/使用方式区域、重复资源路径摘要和独立部署结果卡片。
- [x] “运行状态”日志窗口固定展开，不可折叠，可刷新、滚动，并显示环境检测、提示词投放/还原、Skill、工具和破甲步骤。
- [x] 日志中的成功和失败结果可辨识；破甲 Shell 逐行输出与步骤结果集中显示。
- [x] 客户端环境检测、提示词投放/还原、Skill 与工具安装/卸载、破甲检测/部署/回滚均写入诊断日志并由页面筛选展示。
- [x] 破甲实时逐行日志继续写入诊断日志，并通过 `leila-deploy-log` 事件追加到页面。
- [x] 页面停留期间诊断日志定时刷新，离开页面后停止轮询。
- [x] 日志显示路径和阶段；日志接线不写入提示词正文、密钥、令牌或环境变量值。
- [x] Rust 定向测试、前端 TypeScript 检查、默认 Release 构建通过。

## 验证方式

- Core/manager Tauri 定向测试。
- `npm --prefix apps/claude-codex-pro-manager run check`
- `cargo fmt --check`
- 默认路径 `target/release/claude-codex-pro.exe` 存在且时间为本次构建。
- 未执行：启动桌面页面后的人工点击验证；已通过源码接线、前端检查、Vite 构建、Rust 定向测试和默认 Release 构建验证。

## 非目标

- 不创建新的 Shell 执行通道。
- 不持久化提示词正文或敏感凭据。
