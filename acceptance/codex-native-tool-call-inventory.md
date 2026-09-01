# Codex 原生动态工具调用验收

验证对象：`spec/codex-native-tool-call-inventory.md`

## 通过标准

1. Bootstrap 包含 `codex_native_tool_calls` 只读集合。
2. 每条记录只显示真实调用 ID、线程 ID、工具名称和来源标记。
3. 数据库缺失或 schema 不匹配时返回空集合，不影响任务看板。
4. Rust 单测、TypeScript 检查和 Release 构建通过。
