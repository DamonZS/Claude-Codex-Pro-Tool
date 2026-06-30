# Design QA

prototype: `docs/ui-previews/future-space-manager/index.html`

source:
- `design/context.md`
- `spec/manager-future-space-ui-preview.md`
- user feedback: V1 不行，重新做；内容布局必须合理、不拥挤；不要改变现有前端出现的功能，不要增删。

## Visual

- PASS: V2 使用完整文字侧栏、顶部状态栏、状态卡、主工作区和辅助面板，页面更像真实管理工具，而不是单纯概念海报。
- PASS: 空间感来自透视网格、结构线、悬浮玻璃面板和 3D 控制舱，不再依赖大面积漂浮光球。
- PASS: 桌面截图中概览、工具与插件、会话管理、设置页面文字可读，面板之间留白充足，没有明显拥挤或重叠。
- PASS: 工具与插件、设置页开关改为独立 flex 胶囊控件，圆球在胶囊上下居中。
- PASS: 设置页没有恢复 `Codex 启动参数`、`图片覆盖`、`安全边界`、`用户脚本` 等此前要求删除的旧卡片。
- WATCH: 这是未来空间概念稿，真实落地时仍需按实际数据量重新校准滚动高度、对比度和组件复用方式。

## Behavior

- PASS: hash 路由支持 7 个页面直达：`#overview`、`#supplier`、`#tools`、`#sessions`、`#maintenance`、`#settings`、`#about`。
- PASS: 页面内导航按钮和预览开关已在 HTML 中绑定静态交互；截图验证覆盖每个页面 hash。
- PASS: 未调用真实后端、Tauri API 或外部网络请求。

## Requirements

- PASS: 覆盖概览、供应商、工具与插件、会话管理、维护、设置、关于。
- PASS: 每个页面展示对应页面的代表性核心内容。
- PASS: 本阶段未应用到真实 React/Tauri 前端。
- PASS: 预览文案明确“评审原型”，避免误认为真实工具。

## A11y Sanity

- PASS: 主要按钮、导航项和开关目标尺寸足够大。
- PASS: 主要内容在 1440x1000 Chromium 截图中具备基础可读性。
- WATCH: 未做完整屏幕阅读器测试；真实落地前需补键盘焦点、语义和对比度复核。

## Verification

- `Test-Path docs\ui-previews\future-space-manager\index.html` -> `True`
- 静态检查：`fetch(`、`invoke(`、`__TAURI__` 均为 `False`
- 页面标签检查：概览、供应商、工具与插件、会话管理、维护、设置、关于均存在
- 旧设置卡片关键词检查：未命中 `Codex 启动参数`、`图片覆盖`、`安全边界`、`用户脚本`
- Chromium 截图已生成：
  - `output/playwright/future-space-manager/overview.png`
  - `output/playwright/future-space-manager/supplier.png`
  - `output/playwright/future-space-manager/tools.png`
  - `output/playwright/future-space-manager/sessions.png`
  - `output/playwright/future-space-manager/maintenance.png`
  - `output/playwright/future-space-manager/settings.png`
  - `output/playwright/future-space-manager/about.png`

verdict: READY FOR USER REVIEW — V2 可以用于视觉和布局评审，但尚未被批准应用到真实前端。
