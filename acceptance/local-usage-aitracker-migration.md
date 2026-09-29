# AITRACKER 本地采集器与解析契约迁移验收

对应规格：`spec/local-usage-aitracker-migration.md`

## 通过标准

1. `crates/claude-codex-pro-data/src/local_usage.rs` 提供统一事件、adapter、scanner、session-id 和 aggregate 能力。
2. 同一个结构化 session 输入产生稳定的匿名 ID；输出中不存在原始 session、完整路径或消息正文。
3. JSON/JSONL 映射可读取 Codex/Claude 风格嵌套字段；SQLite adapter 可通过只读查询读取列；坏行/坏查询被跳过并产生诊断；未知字段保持 `unknown`/空值。
4. Token 总量遵循组件合计优先规则，缓存输入、缓存创建、推理输出均计入总量。
5. 相同 session + message/event 身份的快照只保留最大 Token 版本。
6. 聚合快照在无事件时为 `empty`，有事件时为 `real`，并提供来源、模型、项目、日期分组和最近事件。
7. 所有读取均受预算限制；同一 Git 仓库的嵌套 cwd 归并为一个项目，非路径项目引用保持原值且不泄露完整路径；代理记录可以转换到同一事件模型。
8. 通过定向 Rust 测试（含 SQLite 只读行读取）、既有 request_history 测试、前端类型检查和 `git diff --check`。

## 证据

- 测试命令的真实 stdout、退出码。
- 修改文件列表与 diff 检查结果。
- 不要求截图或 UI 外层改动；概览页行为以现有契约测试为准。
