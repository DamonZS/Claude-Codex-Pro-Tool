# 恢复 Codex 原生会话菜单

## 背景

CCP Renderer 注入曾在 Codex 会话行追加删除、导出和项目移动操作组，并通过本地状态隐藏删除过的会话。这会遮挡 Codex 原生会话行交互和右键菜单。

## 目标

- 会话列表完全使用 Codex 原生行、事件和右键菜单。
- 停止 CCP 对会话行的删除、Markdown 导出、项目移动和会话可用性探测增强。
- 清理已运行旧注入留下的操作组、归档导出按钮、隐藏样式和事件监听器。
- 保留供应商、插件、Timeline、对话布局、服务模式等非会话行增强。

## 非目标

- 不删除 Codex 原生会话数据或修改 Codex 原生删除流程。
- 不删除 CCP 管理器自身的会话数据接口；本任务只限制 Codex Renderer 注入层。
- 不修改供应商、启动器和其他非会话行功能。

## 验收

- `sessionRows()` 返回 Codex 原生侧栏行，不读取或应用 CCP 删除缓存和可用性结果。
- 注入启动和扫描会移除旧 `.codex-session-actions`、归档导出按钮、删除确认层和移动弹层，并解除删除文档监听器。
- 不再创建或挂载 CCP 删除、导出、移动按钮，也不发送 `/delete`、`/export-markdown`、`/session-availability` 请求。
- `node --check assets/inject/renderer-inject.js` 和核心注入契约测试通过。
