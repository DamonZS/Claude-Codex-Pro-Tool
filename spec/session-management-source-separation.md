# 会话管理数据源隔离

## 背景

会话管理页同时显示 Codex 会话管理与 Claude 会话管理。当前 Claude 面板复用了 Codex 的 `localSessions` 数据源，导致两个面板的数据库路径、项目和会话内容完全一致，用户无法判断 Claude 会话是否被真实读取。

## 目标

- Codex 会话面板只显示 Codex 本地 SQLite / rollout 会话。
- Claude 会话面板不得复用 Codex 会话数据。
- 若 Claude 真实会话扫描尚未接入，Claude 面板必须显示独立的“待接入/未读取”状态，而不是展示 Codex 内容。
- 刷新历史会话仍刷新 Codex 本地会话与设置，不改变现有删除 Codex 会话能力。

## 非目标

- 本次不实现完整 Claude 会话文件扫描和删除。
- 本次不删除用户会话数据。
- 本次不改变盘古记忆采集层。

## UI 要求

- Codex 面板显示 Codex 数据库路径、候选库数量、会话数、项目和会话列表。
- Claude 面板如果没有独立 Claude 数据源，应显示：
  - 来源：Claude 会话源
  - 状态：待接入
  - 会话数：0 个
  - 空态说明：Claude 会话扫描尚未接入，不会复用 Codex 会话。
- Claude 面板不得出现 Codex 数据库路径和 Codex 项目列表。

## 技术约束

- 保持 `LocalSessionsResult` 作为 Codex 会话结果。
- 前端渲染函数必须显式接收数据源，不允许闭包隐式读取全局 `sessions/localSessions` 后同时渲染 Codex 与 Claude。
- 添加回归测试，防止 Claude 面板再次调用 Codex 数据源。
