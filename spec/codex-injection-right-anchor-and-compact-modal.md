# Codex 注入右侧锚点与紧凑控制舱

## 背景

macOS Codex 顶栏中央用于展示项目或任务名称。CCP 状态入口按左侧文字锚点定位时，会覆盖项目名称；现有盘古控制舱在常见 macOS 窗口中也接近全屏，影响上下文辨认和快速操作。

## 目标

- Windows 与 macOS 的 CCP 状态入口统一定位到右侧顶栏。
- Windows 优先以窗口最小化控件为右边界；macOS 和其他布局以右侧文件打开或工具按钮组为右边界。
- CCP 状态和盘古记忆入口向左排列，不覆盖项目名称或原生按钮。
- 将盘古控制舱改为紧凑弹窗，同时保留全部导航、设置和滚动能力。

## 功能要求

- 右侧锚点识别优先匹配最小化按钮的 `aria-label`、`title` 或测试标识。
- 找不到最小化按钮时，从 Header 右半区的原生按钮组选择最靠左的可见按钮作为锚点。
- 状态入口位于锚点左侧并限制在窗口可视范围内；盘古记忆入口位于状态入口左侧。
- 找不到 Header 或右侧按钮时使用右侧安全边距回退，不能回到项目名称区域。
- 控制舱桌面上限收缩，Header、侧栏、Hero、能力条和设置卡片同步紧凑化。
- 小于 900px 和 720px 的既有响应式布局继续生效。

## 技术约束

- 仅修改注入脚本、定向测试和本规格对应的验收文档。
- 不新增依赖，不修改后端接口和设置字段。
- 不改变弹窗关闭、设置切换、服务等级、推荐、支持和联系入口的行为。

## 交付范围

- `assets/inject/renderer-inject.js`
- `crates/claude-codex-pro-core/tests/cdp_bridge.rs`
- `spec/codex-injection-right-anchor-and-compact-modal.md`
- `acceptance/codex-injection-right-anchor-and-compact-modal.md`
