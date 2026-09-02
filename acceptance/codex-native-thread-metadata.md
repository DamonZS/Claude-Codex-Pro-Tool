# Codex 原生会话元数据投影验收

验证对象：`spec/codex-native-thread-metadata.md`

## 通过标准

1. 原生会话查询保留稳定字段和存在时的归档、置顶、模型、来源、分支、项目关联字段。
2. 缺少可选列的兼容 schema 不会导致读取失败。
3. 投影不包含提示词、rollout 路径或会话正文。
4. Rust 原生工作区测试全部通过，格式和 diff 检查通过。

## 验证方式

- `cargo test -p claude-codex-pro-core multica_workspace --lib -- --nocapture`
- 使用本机只读 SQLite 检查 `threads` 表字段与进程启动路径。
