# 修复 Windows Manager UI 回归测试

## 背景

提交 `a102ece` 为 Codex/Claude 会话上下文增加了合法的模态详情，并且工具页已经使用统一工具资产面板和更明确的仓库配置文案；`windows_subsystem` 中三条旧的源码字符串断言仍绑定此前结构，导致两个 Windows GitHub Actions Job 在 Rust tests 阶段失败。

## 目标

- 让 UI 回归测试匹配当前已规格化的工具页与会话上下文结构。
- 继续保留工具管理、仓库状态和非模态通知的回归护栏。
- 恢复 Windows 构建流水线的 Rust tests 阶段。

## 非目标

- 不修改 Manager 运行时代码、UI 布局或用户可见文案。
- 不放宽会话上下文的无障碍要求。
- 不改 GitHub Actions 工作流。

## 功能与技术要求

- 工具页测试应验证 `UnifiedToolInventoryPanel` 及其 Codex/Claude 双端操作，不再要求已移除的 `ContextManagerPanel`。
- 插件仓库测试应验证当前“配置已写入/配置未写入”状态文案。
- 布局测试只禁止通知系统回退为模态通知，不得全局禁止会话上下文所需的 `role="dialog"` 和 `aria-modal`。
- 只修改测试和对应规格/验收文档。

## 交付范围

- `apps/claude-codex-pro-manager/src-tauri/tests/windows_subsystem.rs`
- 本规格及对应验收标准。
