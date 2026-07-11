# 验收标准：Codex 会话上下文查看器

验证对象：`spec/codex-session-context-viewer.md`

## 通过/失败标准

1. Codex 会话行调用独立的 `loadCodexSessionContext`，删除按钮仍只执行删除且不触发打开。
2. `load_codex_session_context` 已注册，并返回标题、项目、来源、分页元数据与有序消息。
3. 后端拒绝不在重新发现的 Codex 数据库候选集合中的 `dbPath`，不接受任意路径。
4. 初次加载最近消息；加载更早消息后无重复且顺序正确。
5. 详情具备加载、空、错误/重试状态，以及按钮、遮罩、Escape 关闭方式。
   - Codex 上下文中的 `assistant` 角色必须显示为 `Codex`，不得显示为 `Claude`。
6. Claude 会话上下文现有行为和 Codex 删除/刷新行为不回退。
7. 前端类型检查、生产构建、数据层定向测试和 Manager UI 回归测试通过。

## 必需验证与证据

```powershell
cargo test -p claude-codex-pro-data --manifest-path Cargo.toml
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem -- --nocapture
```

证据为命令退出码与测试摘要，并检查实际 diff。无法运行的命令必须说明原因。

## 非目标检查

- 不要求修改数据库 schema。
- 不要求渲染图片、工具调用参数或隐藏思考内容。
- 不要求自动打开 Codex 官方应用中的对应任务。
