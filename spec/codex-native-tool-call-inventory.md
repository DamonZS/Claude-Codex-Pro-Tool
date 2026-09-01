# Codex 原生动态工具调用投影

## 目标

从当前用户的 Codex SQLite 状态库只读投影 `thread_dynamic_tools`，让“我的任务”显示真实工具调用记录及所属线程。

## 约束

- 仅打开 `codex_session_db_paths_from_home` 返回的数据库，使用只读连接。
- 字段按实际 schema 探测；缺少线程、名称或调用 ID 时安全降级为空集合。
- 投影数据不可编辑、不可用于伪造执行成功，也不读取参数中的凭据。
