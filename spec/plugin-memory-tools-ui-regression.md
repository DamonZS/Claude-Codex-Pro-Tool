# 插件仓库、盘古记忆与工具页 UI 回归修复

## 背景

用户反馈当前 Codex 注入后的插件页仍未完整解锁官方插件仓库展示；盘古记忆进入对话后只记录到类似“codex:你好”的标题文本，后台和注入标识均没有真实对话记录；管理工具所有页面顶部仍显示“后端链接”胶囊；工具与插件页仓库状态展示被截断，无法明确看到官方仓库状态；下方 MCP / Skills / Plugins 列表中的开关、编辑和删除按钮仍未对齐。

这些问题会让用户误判插件仓库修复是否生效、盘古记忆是否真正监听对话，以及管理工具页面是否加载了最新版本。

## 目标

本次要完成：

- Codex 注入脚本对插件 marketplace 响应的扩展必须让官方 `openai-curated` / `openai-api-curated` 插件结果在搜索/过滤场景中完整可见，不被隐藏过滤器误删。
- 盘古记忆监听必须从 Codex 主对话区域提取真实用户/助手消息，不得把左侧会话标题或窗口标题当作对话内容写入。
- 管理工具所有页面顶部去掉“后端链接 127.0.0.1:xxxx”胶囊。
- 工具与插件页的 Codex / Claude 仓库卡必须明确显示官方仓库状态，长文本不得被截断到无法识别。
- 工具与插件页 MCP / Skills / Plugins 条目中的开关、编辑、删除控件必须右侧对齐、尺寸稳定、互不重叠。

本次不包含：

- 自动安装具体官方插件。
- 删除用户本地记忆数据库内容。
- 改变 Codex / Claude 登录状态。
- 重做工具与插件页整体信息架构。

## 用户视角描述

用户打开 Codex 插件页并搜索插件时，应能看到官方插件仓库中的完整匹配结果。用户进入任意 Codex 对话后，盘古记忆只在检测到真实对话消息时上报会话摘要，不再把侧边栏标题误识别为对话。用户打开管理工具任意页面时，顶部只保留页面标题和操作按钮，不再看到后端链接胶囊。工具与插件页顶部两张仓库卡应清楚显示 Codex 官方仓库与 Claude 官方/Ponytail 仓库的状态，下方开关和操作图标整齐排列。

## 功能要求

- 注入脚本在处理 Codex 插件 marketplace 数据时，必须为 `openai-curated` 生成稳定别名并扩展搜索响应，避免官方仓库被隐藏过滤器排除。
- Codex 仓库修复逻辑必须同时兼容 `openai-curated` 与既有 Codex 安装记录中常见的 `openai-api-curated`，避免管理工具显示已注册但 Codex 插件页仍无法识别官方仓库。
- Codex 插件页搜索必须按关键词分词匹配插件名称、描述、分类、interface 描述和 keywords，不得要求搜索词完整连续出现。
- 注入脚本的盘古记忆采集必须限定在主内容区、文章或消息容器内，并过滤侧边栏、导航、标题栏、插件页、设置页等非对话文本。
- 当提取不到真实用户消息时，不得调用 `/memory/session` 写入空摘要或标题摘要。
- 管理工具 header 不得渲染 `backend-chip` / “后端链接”。
- Codex 仓库卡必须显示至少：官方仓库名称、配置状态、本地目录。
- Claude 仓库卡必须逐条显示官方仓库和 Ponytail 仓库的 configured 状态。
- 仓库状态行允许换行或使用垂直列表，不得依赖单行省略导致关键信息不可见。
- 上下文条目行的右侧 controls 必须使用固定宽度/稳定布局，开关、编辑、删除按钮不得压缩到重叠。

## UI / 交互要求

- 顶部 header 删除后端链接后，启动/重启按钮和刷新按钮保持右侧排列。
- 仓库卡内部状态值应可读，长路径可以截断，但仓库名和 configured 状态必须可见。
- 开关保持现有视觉风格，但在条目行中有固定宽度，并与编辑/删除图标居中对齐。

## 数据与接口要求

- 继续复用现有 `load_codex_plugin_marketplace_status`、`repair_codex_plugin_marketplace`、`load_claude_desktop_marketplace_status`、`repair_claude_desktop_marketplaces`。
- 继续复用现有 `/memory/session`，但前端注入侧必须传入真实对话提取结果。
- 不新增远程依赖。

## 技术约束

- 不引入新依赖。
- 不修改 Claude 中文注入脚本。
- 不删除本地用户数据。
- 保持最小必要改动，并补充回归测试。

## 交付范围

- `assets/inject/renderer-inject.js`
- `apps/claude-codex-pro-manager/src/App.tsx`
- `apps/claude-codex-pro-manager/src/styles.css`
- `crates/claude-codex-pro-core/tests/cdp_bridge.rs`
- `apps/claude-codex-pro-manager/src-tauri/tests/windows_subsystem.rs`
- 本规格文档与对应验收标准
