# 会话数据库索引验收标准

对应规格：`spec/session-database-index.md`

## 通过标准

- `npm --prefix apps/claude-codex-pro-manager run check` 通过，且演示组件使用现有 `SessionIndex` 与 `PagedResult` 类型。
- `cargo test -p claude-codex-pro-core session_index -- --nocapture` 通过。
- 演示组件调用现有 `query_sessions(filter, page, page_size)` 和 `scan_sessions` Tauri 命令，字段与后端序列化结构一致。

## 验证方式

运行上述命令并记录退出状态；检查组件导入、调用参数和展示字段与 `src/api/sessionIndex.ts` 及 Rust 命令定义一致。

## 非目标

- 不改变会话索引数据库 schema、用户数据位置或索引业务流程。
- 不将未挂载的开发演示组件接入生产页面。
