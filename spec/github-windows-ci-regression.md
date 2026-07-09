# GitHub Windows CI 构建失败修复

## 背景

GitHub Actions 的 Windows 构建在 `windows_subsystem` Rust 回归测试阶段失败。截图中的失败项显示为 `Windows Rust tests failed`，本地复现后定位到 `manager_window_and_ops_console_layout_stay_usable` 仍断言概览页盘古记忆旧入口。

此前盘古记忆已经调整为独立导航页；概览页盘古记忆卡只保留开关、运行状态、Codex 注入、对话监控与现有动画效果。因此旧测试中关于 `查看/编辑经验教训`、`提炼经验教训`、`memory-overview-matrix`、`memory-overview-actions` 的概览页断言已经过期，会导致 Windows CI 阻断。

## 目标

本次要完成：

- 修复 Windows `windows_subsystem` 回归测试，使其匹配当前概览页盘古记忆设计。
- 保留对概览页精简盘古记忆卡的护栏：开关、运行状态、Codex 注入、对话监控、动画组件必须存在。
- 保留独立盘古记忆页的提炼、查看、编辑能力，不把旧入口重新塞回概览页。
- 不修改 GitHub Actions 发布流程的业务逻辑，除非确认为 workflow 本身错误。

本次不包含：

- 不重构盘古记忆 UI。
- 不改变会话管理页、供应商页和 Claude 中文注入逻辑。
- 不新增发布产物或改变版本策略。

## 用户视角描述

用户推送代码后，GitHub Windows 构建应通过。概览页继续保持精简盘古记忆状态卡，不因为修 CI 而重新出现之前删除的经验教训按钮或详情入口。

## 功能要求

- `manager_window_and_ops_console_layout_stay_usable` 不再断言概览页存在旧的经验教训操作入口。
- 测试应断言概览页盘古记忆卡包含：
  - `盘古记忆开关`
  - `运行状态`
  - `Codex 注入`
  - `对话监控`
  - `MemoryActivityWave`
- 测试应断言概览页盘古记忆卡不包含：
  - `查看/编辑经验教训`
  - `提炼经验教训`
  - `memory-overview-matrix`
  - `memory-overview-actions`
- 独立盘古记忆页仍保留会话经验教训注入手册与提炼入口。

## 技术约束

- 优先修改 `apps/claude-codex-pro-manager/src-tauri/tests/windows_subsystem.rs`。
- 不为了让测试通过而回滚产品 UI。
- 不删除现有用户数据，不改数据库结构。

## 交付范围

- `apps/claude-codex-pro-manager/src-tauri/tests/windows_subsystem.rs`
- `spec/github-windows-ci-regression.md`
- `acceptance/github-windows-ci-regression.md`
