# 总览真实用量与原生采集语义核对

## 背景和目标

用户反馈总览的模型、Token、会话、缓存命中率和趋势与 AITracker 不一致。统一快照目前受到 4096 条事件截断，Claude 同时经过旧读取器和通用 adapter，Codex adapter 未识别原生 token_count。

本次只修改后端统一采集与对应测试；不修改页面结构、外层框架或液态玻璃材质，不保存正文、密钥或修改用户数据。

## 数据要求

- Codex 和 Claude 的 JSONL adapter 复用原生解析器，读取 session/project/model 上下文，Claude assistant message 的重复快照只计一次。
- 旧 request_history 的 4096 条请求列表契约保留；统一 usage_snapshot 使用独立、明确的 100000 事件预算。达到预算必须返回截断诊断，不把部分覆盖描述成全量。
- Codex 原始 input_tokens 已包含 cached_input_tokens，统一事件的非缓存输入为两者非负差值；总量按非缓存输入、缓存读取、缓存创建、输出、推理输出合计，对齐 AITracker scanner.server.ts 的 codexEventFromRecord。
- 相同 source/session/message 身份的快照去重，不把时间或累计 Token 值放进具有稳定 message ID 的身份键。
- JSONL 逐行读取，串行扫描；保持 1200 文件、30000 目录条目、256 MiB 单文件预算。SQLite 按行查询，不以数据库文件大小作为 JSON 文件预算而跳过。
- 跨文件扫描及通用 JSONL 行读取时，中间事件集合超过两倍事件预算即去重、保留最新事件并返回截断诊断，避免事件随文件数无限累积。
- 数据核对使用相同时间范围、来源和模型分组。打印仅包含聚合数量和 Token，不导出消息或密钥。

## 验证与交付

提供原生 Codex 缓存语义、Claude 同消息快照、超过 4096 条后的旧模型保留、重复来源去重测试。运行 data crate 定向测试，报告实际本机只读采集数量和时间；最终默认 Release 由主代理统一构建。
