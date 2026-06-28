# Codex 注入状态标识可见性与面板关闭隔离

## 背景

Codex 窗口标题栏为浅色时，注入状态标识中的文字颜色不够接近原生窗口文字，导致用户几乎看不见 “CCP / 盘古记忆” 一类状态信息。同时，用户反馈 CCP 面板中点击关闭后会导致面板里的开关状态被一起关闭，说明关闭动作与开关点击处理需要更明确隔离。

## 目标

本次要完成：

- Codex 顶部注入标识文字颜色必须直接使用截图中原生菜单“视图”同类文字色 `#a9a4a9`，避免浅色窗口下不可见。
- 状态通过圆点颜色表达，文字本身不使用红/绿等状态色。
- CCP 面板关闭按钮只关闭面板，不触发任何设置开关、不写入 localStorage、不调用后端设置保存。
- 点击面板遮罩关闭时也不能触发开关。
- CCP 面板主题颜色必须与管理工具一致，使用浅色运维面板色系：白色面板、深色正文、灰蓝辅助文字、青绿色主色和浅边框。
- CCP 面板中不再显示或触发“用户脚本”管理入口。

本次不包含：

- 删除用户脚本后端路由或用户本地脚本数据。
- 改动盘古记忆业务逻辑。
- 改动后端设置字段。

## 用户视角描述

用户打开 Codex 浅色窗口时，标题栏上的 CCP 注入标识文字应像 “文件 / 编辑 / 视图 / 帮助” 这些窗口文字一样清晰可见。用户打开 CCP 面板后，点击右上角关闭或遮罩关闭，只应关闭面板，不能把页面功能增强、插件解锁、盘古记忆等开关一起关掉。

## 功能要求

- 浮动状态条容器不得用媒体查询强行把文字设成暗色或亮色，必须直接使用 `#a9a4a9`。
- 盘古记忆注入标识文字也必须使用同一文字色，计数文字继承该颜色。
- 关闭按钮事件必须 `preventDefault` 和 `stopPropagation`。
- 遮罩 click 处理必须在关闭后直接 return，避免继续执行同一 click handler 中的开关逻辑。
- 开关逻辑只允许响应 `[data-claude-codex-pro-setting]` 按钮本身或其子元素的点击。
- CCP 面板必须移除 `userScripts` tab、`userScripts` panel、用户脚本开关、用户脚本列表和重载按钮；打开面板时不得再请求 `/user-scripts/list`。

## 技术约束

- 不引入新依赖。
- 只改注入脚本和定向测试。
- 使用现有 `cdp_bridge` 字符串断言验证关键行为。

## 交付范围

- `assets/inject/renderer-inject.js`
- `crates/claude-codex-pro-core/tests/cdp_bridge.rs`
- `spec/codex-injection-status-visibility-and-modal-close.md`
- `acceptance/codex-injection-status-visibility-and-modal-close.md`
