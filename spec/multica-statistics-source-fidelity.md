# 统计来源与能力边界

## 背景

上游 Multica 的 usage dashboard 从 `task_usage` 与 `task_usage_hourly` 提供 token、成本、日期、时区、项目和按智能体聚合。本地 CCP 当前只有执行控制面与任务 JSON，不能推导这些 usage 字段。

## 目标

- 统计集合必须明确标记真实来源为 `local_control_plane`。
- 返回机器可读的限制列表，防止 UI 或调用方把控制面计数当作上游 usage parity。
- 不生成缺失的 token、provider/model、成本或时间序列数据。

## 功能要求

统计项必须包含：

- `source: "local_control_plane"`
- `limitations`，至少包含 `token_usage_unavailable`、`provider_model_breakdown_unavailable`、`time_series_unavailable`、`project_filter_unavailable`
- 现有执行、绑定、任务状态计数保持兼容

## 非目标

本规格不伪造上游 `/api/dashboard/usage/*`，也不从任务文本估算 token 或成本。
