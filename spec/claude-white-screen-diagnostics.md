# Claude 白屏诊断与兜底修复

## 背景

用户反馈运行中的 Claude 出现白屏。项目历史上曾提供 Claude 汉化包装窗口能力，但前端已不再展示“包装 WebView”入口；后台命令仍可能被旧状态、快捷入口或内部调用触发。当前后台包装窗口直接以外部 URL `https://claude.ai/new` 创建 WebView，若 Claude 页面在 WebView2 登录、网络、CSP 或兼容性层阻塞，用户看到的是无解释白屏。

## 目标

本次要完成：

- 检查运行中 Claude/管理工具进程状态，确认白屏相关链路。
- 保留官方 Claude Desktop 启动链路，不修改 Claude 中文注入补丁。
- 对后台 Claude 汉化包装窗口增加本地诊断壳与可见兜底，避免纯白屏无反馈。
- 兜底页面提供“在浏览器打开 Claude”入口，并保留插件中心自定义协议导航拦截。
- 增加可验证测试，防止再次退回纯外部 WebView 白屏。

本次不包含：

- 不修改官方 Claude Desktop 安装文件。
- 不恢复前端“包装 WebView”入口。
- 不处理 Claude 账号、网络、腾讯/代理等外部服务状态。
- 不删除用户配置或重置 Claude/Codex 数据。

## 用户视角描述

如果旧入口或内部命令打开 Claude 汉化窗口，用户不应再看到空白窗口。窗口应先显示本地诊断提示、加载状态和外部浏览器打开入口；即使 Claude 页面无法在 WebView 中渲染，也能看到原因提示和下一步操作。

## 功能要求

- `open_claude_chinese_window` 不应直接以 `WebviewUrl::External("https://claude.ai/new")` 创建白屏风险窗口。
- 应使用本地 HTML 壳作为窗口内容。
- 本地 HTML 壳必须包含：
  - 明确的“Claude 加载中”状态。
  - 白屏/加载失败说明。
  - `https://claude.ai/new`。
  - “在浏览器打开 Claude”按钮或链接。
- 现有 `claude-codex-pro://plugin-hub` 导航拦截仍保留。

## UI / 交互要求

- 页面不追求新增复杂 UI，只需清楚、可读、有操作入口。
- 不恢复管理工具前端的“包装 WebView”按钮。

## 技术约束

- 最小改动。
- 不新增依赖。
- 不改 Claude 中文注入脚本。
- 不杀 Codex 主进程。
- 构建时若 manager 被占用，可终止 `claude-codex-pro-manager.exe` 后重建。

## 交付范围

- 缺陷规格与验收标准。
- 后台 Claude 包装窗口白屏兜底修复。
- 定向测试。
- 前端检查与 manager 构建验证。
