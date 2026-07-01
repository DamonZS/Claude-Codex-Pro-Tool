# Design QA

prototype: `docs/ui-previews/manager-interface-redesign/index.html`

source:
- `spec/manager-interface-redesign-preview.md`
- `acceptance/manager-interface-redesign-preview.md`
- user feedback: previous dark engineering version was "太丑了"
- references: Apple pearl surfaces, Stripe-like product dashboard polish, existing manager IA

## Visual

- PASS: V3 改为 Pearl Command Deck：浅色珠光画布、深色指挥舱模块、蓝青主操作与更强产品感。
- PASS: 不再使用上一版全黑工程后台风格，整体更明亮、更高级，侧栏、状态卡与主面板层级更清晰。
- PASS: 概览页使用深色智能控制室承载状态流，保留高级感和可读性。
- PASS: 工具与插件页右侧操作槽固定，开关圆球在胶囊上下居中，编辑按钮为单只倾斜笔视觉。
- PASS: 设置页没有恢复 `Codex 启动参数`、`图片覆盖`、`安全边界`、`用户脚本` 等旧卡片。
- WATCH: 这是独立评审原型；若应用到真实前端，需要逐页映射现有 React 组件，不能直接替换业务树。

## Behavior

- PASS: hash 路由覆盖 7 个页面：`#overview`、`#supplier`、`#tools`、`#sessions`、`#maintenance`、`#settings`、`#about`。
- PASS: 页面内导航和开关都绑定了静态交互，不执行真实系统命令。
- PASS: 未调用 `fetch`、Tauri `invoke` 或 `window.__TAURI__`。

## Requirements

- PASS: 覆盖概览、供应商、工具与插件、会话管理、维护、设置、关于。
- PASS: 每个页面展示对应页面的代表性核心内容。
- PASS: 本阶段未应用到真实 React/Tauri 前端。
- PASS: 预览文案明确“静态评审稿”，避免误认为真实工具。

## A11y Sanity

- PASS: 主要按钮、导航项和开关目标尺寸满足基础可点性。
- PASS: 1440x1000 Chromium 截图中主要文本可读，按钮未遮挡长文本。
- WATCH: 未做完整屏幕阅读器测试；真实落地前需补键盘焦点、语义和对比度复核。

## Verification

- `Test-Path docs\ui-previews\manager-interface-redesign\index.html` -> `True`
- 静态检查：`fetch(`、`invoke(`、`__TAURI__` 均为 `False`
- 旧设置卡片关键词检查：`Codex 启动参数`、`图片覆盖`、`安全边界`、`用户脚本` 均未命中
- 路由定义检查：`overview`、`supplier`、`tools`、`sessions`、`maintenance`、`settings`、`about` 均存在
- Chromium 截图已生成：
  - `output/playwright/manager-interface-redesign/overview.png`
  - `output/playwright/manager-interface-redesign/supplier.png`
  - `output/playwright/manager-interface-redesign/tools.png`
  - `output/playwright/manager-interface-redesign/sessions.png`
  - `output/playwright/manager-interface-redesign/maintenance.png`
  - `output/playwright/manager-interface-redesign/settings.png`
  - `output/playwright/manager-interface-redesign/about.png`

verdict: READY FOR USER REVIEW — 适合你评审视觉方向与页面布局，尚未批准应用到真实前端。
