# Codex 内嵌工作区自定义任务状态目录验收

对应规格：`spec/codex-custom-issue-status-catalog.md`。

1. 本地工作区总能读取七个系统状态，且它们分别映射到七个固定类别。
2. 自定义状态必须具有唯一、稳定的 key、有效名称、`#rrggbb` 颜色和固定类别；无效类别或颜色被拒绝。
3. 系统状态不能被重命名、重分类或归档；自定义状态不能在更新时改 key 或类别。
4. Issue 保存自定义状态时，其 `status_category`/`status_name` 与目录一致；不存在或已归档状态不会被接受为新写入。
5. 看板、列表、表格和泳道按状态类别展示自定义状态，且显示目录名称；未知历史键保持可见且不被静默改写。
6. `cargo test -p claude-codex-pro-core multica_workspace --lib -- --nocapture`、`node --check assets/inject/renderer-inject.js`、前端检查与 Vite 构建通过。
