# 我的任务多视图模式验收

对应规格：`spec/multica-my-tasks-view-modes.md`

## 通过标准

- 源码包含 board/list/table/swimlane 四个模式入口，且按钮使用 `aria-pressed` 表示当前模式。
- list、table、swimlane 使用当前筛选后的同一任务集合渲染。
- board 的拖放和创建行为保持可用。
- `my-issues` 渲染路径不调用原生活动列表，源码中首屏不得出现 `当前活动` 或 `来源：Codex 本机活动投影`。
- `my-issues` 使用深色背景、七列横向看板、紧凑列头和卡片化任务样式，与上游页面结构一致。
- `node --check assets/inject/renderer-inject.js` 通过。
- `npm --prefix apps/claude-codex-pro-manager run check` 与 `vite:build` 通过。

## 非目标

- 不要求页面 Host 可用；Host 不可用时仍按真实状态显示本地任务和不可用提示。
