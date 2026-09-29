# 总览用量采集验收

对应规格：`spec/overview-usage-parity.md`。

1. Codex 原生事件包含正确模型、匿名会话和项目末段；原始输入 100、缓存 80、输出 20、推理 7 在统一事件中为非缓存输入 20、总量 127。
2. Claude 原生 adapter 和旧 reader 的同一消息在统一快照中只计一次；同 session/message 的累计快照取较大值。
3. 使用包含超过 4096 条记录的固定 fixture，统一 adapter 保留旧模型；请求列表旧接口仍遵循调用方指定 limit。
4. 超出事件预算产生 Truncated 诊断；JSONL 不构建完整原始 JSON Value 数组。
5. SQLite 使用只读查询、行预算；未知字段不伪造。
6. `cargo test --release -p claude-codex-pro-data --lib local_usage -- --nocapture` 和 request_history 集成测试通过；只读本机 smoke 仅打印聚合统计。
7. 最终交付报告列出测试 stdout、退出码、改动文件、数据差异根因和尚未覆盖的专用 reader，不宣称所有专用 schema 已完全等价。
