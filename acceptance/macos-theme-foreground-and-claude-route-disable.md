# 验收标准：macOS 浅色主题可读性与 Claude Desktop 路由关闭

验证对象：`spec/macos-theme-foreground-and-claude-route-disable.md`

## 验收项

1. `Codex Dream Skin - macOS` 从目录和 ZIP 编译出的运行时 CSS 都包含浅色主题兼容标记、Codex token / VS Code 前景色变量，以及不依赖 `main.main-surface` 的稳定首页背景选择器。
2. 已安装的旧版浅色主题也通过活动主题载荷生成路径获得兼容 CSS；默认主题和深色主题不追加该标记。
3. 两套 Dream Skin 的运行时 CSS 将首页指纹后的旧 `> div:first-child` 布局锚点升级为 `> div:has([data-feature="game-source"])`，载荷中不再保留会把 Codex 26.721 前置空节点撑满视口的旧组合选择器。
4. macOS 主题重启后可见头图、品牌标识、首页标题、建议卡片和输入区；主区域不再是整屏空白，整体层级与参考截图一致。
5. 关闭活动 Claude Desktop 路由的前端分支先确认官方模式恢复成功，再调用 `saveSupplierSettings` 持久化 `nextProfiles`，保存成功后才显示关闭完成提示。
6. 关闭后 Profile 的 `routeEnabled` 为 `false`，`claudeDesktopMode` 为 `direct`，对应配置元数据同步为 Direct。
7. `routeEnabled=false` 时 Claude Desktop 模型目录为空、健康状态不为 ready，消息代理在发起上游请求前返回路由已关闭错误；重新开启后保持现有代理行为。
8. 验证 Claude Desktop 活动供应商模型目录归属的测试必须显式开启该 Profile 的路由，避免用默认关闭状态错误断言非空目录。
8. Core 定向主题/代理测试、Manager 路由契约测试、TypeScript 检查、前端构建、Rust 格式检查和默认 Release 构建通过。

## 非范围

- 不删除已有 Claude Desktop 第三方 Profile。
- 不改变深色 Codex 主题的调色板。
- 不重置用户的供应商、API Key 或活动供应商选择。
