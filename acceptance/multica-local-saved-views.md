# Multica 我的任务本机保存视图验收

对应规格：`spec/multica-local-saved-views.md`

## 通过标准

- 语法检查通过：`node --check assets/inject/renderer-inject.js`。
- 管理器类型检查通过：`npm --prefix apps/claude-codex-pro-manager run check`。
- Vite 生产构建通过：`npm --prefix apps/claude-codex-pro-manager run vite:build`。
- 任务工具栏显示本机视图选择、保存和当前视图删除控件。
- 筛选器包含“工作中”，且只显示存在本地非终态 Codex 执行绑定的任务。
- 保存后刷新页面，选择该视图可恢复 scope、显示模式和紧凑布局。
- localStorage 中的无效记录不会导致页面崩溃。
- `git diff --check` 无错误。

## 证据

- 命令输出和构建产物路径由交付记录提供。
- 手动 UI 验证需在 Codex 页面打开“我的任务”后执行保存、刷新、恢复、删除流程。

## 非目标

- 不要求与上游服务端 `IssueView` 同步。
