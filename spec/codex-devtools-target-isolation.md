# Codex DevTools 目标隔离

## 背景

Codex 注入菜单中的“打开 DevTools”必须只检查当前 Codex renderer。现场截图显示，调试端口没有 Codex renderer 时，旧代码把 Claude Desktop 的 `app://localhost/epitaxy` 页面当作任意可注入页面打开，造成 Claude Developer Tools 被误打开。

## 目标

- `/devtools/open` 仅选择 `is_codex_page_target` 认可且具备 WebSocket 的 CDP 页面。
- 没有符合条件的 Codex 页面时返回明确错误，且不调用系统 URL 打开器。
- 不改变 Claude Desktop 自身的显式启动或调试能力；不改变 Codex 注入、任务工作区、供应商和 UI 结构。

## 技术约束

- 复用 `claude_codex_pro_core::cdp::pick_injectable_codex_page_target`，禁止为 DevTools 路由保留“任意 page”回退。
- 仅修改 Launcher 的 Codex DevTools 路径和对应回归测试。

## 交付范围

- Launcher 严格目标选择。
- Claude-only CDP target 的回归测试。
- 对应验收文档。
