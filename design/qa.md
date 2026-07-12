# Codex 注入盘古控制舱设计 QA

prototype: `assets/inject/renderer-inject.js`  
source: `design/context.md` + `design/audit.md` + `design/ideate.md` + `spec/codex-injection-control-deck-redesign.md`

## 视觉

- ✅ 已移除 `claude-codex-pro-comic-shell`、`POWER PANEL`、漫画字体、纸张网点和粗黑偏移阴影。
- ✅ 已建立独立控制舱 token：深石墨表面、青绿能量色、琥珀诊断色、细边框和低对比网格。
- ✅ Header 已展示 CCP 标识、`PANGU LOCAL CONTROL DECK`、版本与后端状态。
- ✅ 桌面为左侧场景导航、右侧独立滚动工作区；`720px` 以下切换为横向导航和单列。
- ✅ 首页明确展示本机运行、模型桥接、盘古记忆和可审查回退四项能力，不伪造实时数量。
- ⚠️ 当前环境无法用 headless Edge 产出截图，真实 Codex 宿主中的字体渲染、对比度和 100%/125%/150% 缩放仍需人工截图复核。

## 行为

- ✅ 遮罩使用 `z-index: 2147483647` 并附加到 `document.body`，建立独立 stacking context。
- ✅ 打开控制舱前发送 Escape keydown/keyup，并让活动元素失焦，避免模型菜单继续保持活动。
- ✅ home、recommendations、support、contact 的现有 panel 和事件契约保持不变。
- ✅ 关闭按钮继续阻止事件传播；点击遮罩关闭后立即 return，不会误切换设置。
- ✅ 67 项 `cdp_bridge` 集成测试全部通过。
- ⚠️ 在真实 Codex 中“先展开模型菜单再打开控制舱”的交互截图尚未完成。

## 需求符合度

- ✅ 已按“模型与插件通道 / 会话与工作流 / 本地运维与诊断”分区长列表。
- ✅ 未修改 bridge API、设置字段、Codex 原生模型菜单、管理工具页面或发布行为。
- ✅ 未引入新依赖、网络字体或不可验证能力。
- ✅ 旧漫画规格已由盘古控制舱规格与验收文档替代。

## 可访问性

- ✅ dialog 保留 `role="dialog"`、`aria-modal="true"` 和可访问名称。
- ✅ 关闭按钮为 40×40px；按钮、输入框和链接提供 `focus-visible` 轮廓。
- ✅ 提供 `prefers-reduced-motion` 降级规则。
- ⚠️ 焦点圈定、关闭后的焦点恢复、屏幕阅读器朗读和精确 WCAG 对比度仍需要真实宿主人工验证。

## 验证证据

- `node --check assets/inject/renderer-inject.js`：通过。
- `cargo fmt --check`：通过。
- `cargo test -p claude-codex-pro-core --manifest-path Cargo.toml --test cdp_bridge -- --nocapture`：67 passed。
- `npm --prefix apps/claude-codex-pro-manager run check`：通过。
- `npm --prefix apps/claude-codex-pro-manager run vite:build`：通过。
- `git diff --check`：通过。

## 结论

**READY FOR HOST VISUAL CHECK** — 代码、契约测试、类型检查和构建均通过，无已知 HIGH 级规格偏差。发布前剩余门槛是使用真实 Codex 完成模型菜单覆盖、缩放、键盘焦点与截图复核。
