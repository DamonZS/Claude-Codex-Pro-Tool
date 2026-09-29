# AITRACKER 能力接入验收

对应规格：`spec/aitracker-capability-integration.md`

## 通过标准

1. Agent registry 的工具数量与 AITRACKER manifest 一致，包含能力和平台字段，未知本地来源标记为未检测。
2. 统一会话查询能从真实 `usage_snapshot.details` 生成匿名 session 摘要，按 Agent、状态、关键词筛选并分页；无事件时返回空态。
3. 工具调用分析只返回工具语义字段和匿名 session，不包含 prompt、参数、输出、凭证和完整路径。
4. Agent 详情的 Token、会话和工具排行来自同一快照，不能使用硬编码示例数字。
5. 蒸馏任务能创建、查询、取消候选；候选正文经过路径/凭证裁剪并要求审批后才可持久化。
6. 既有 Codex/Claude 会话删除、上下文加载、概览页和液态玻璃外层行为无回归。

## 验证方式

- `cargo test -p claude-codex-pro-data --lib local_usage`
- `cargo test -p claude-codex-pro-manager --lib`
- `npm --prefix apps/claude-codex-pro-manager run check`
- `npm --prefix apps/claude-codex-pro-manager run vite:build`
- `git diff --check`

## 非目标检查

- 不要求复制 AITRACKER 页面或 Node 服务。
- 不要求在没有本地记录时显示非零数据。
