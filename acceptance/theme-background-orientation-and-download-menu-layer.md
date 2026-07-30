# 验收标准：主题背景方向兼容与下载菜单层级

验证对象：`spec/theme-background-orientation-and-download-menu-layer.md`

## 验收项

1. `1280×720`、`1920×1200` 和 `1200×1920` Manager 背景均通过校验。
2. 短边小于 `720` 或长边小于 `1280` 时拒绝，并显示长边/短边要求。
3. Codex 主题下载菜单通过 React Portal 挂载到 `document.body`，使用原生模态 `dialog.showModal()` 进入浏览器 Top Layer。
4. 下载菜单不受主题卡片 stacking context 或 macOS WebKit 合成层影响；浅色和深色模式均使用完全不透明背景，最大高度内可滚动，并支持点击透明 backdrop 和 Escape 关闭。
5. “下载主题”使用标准 `Button` 描边样式，下载箭头和文字同处一行，按钮边框、背景、高度与相邻操作按钮一致。
6. 桌面布局下整个主题操作组相对左侧标题信息垂直居中，刷新、DIY 主题、制作指南、下载主题和导入主题按钮自身内容均上下居中；窄屏布局保持可换行。
7. Core 主题测试、Manager 契约测试、类型检查、前端构建和 Release 构建通过。
