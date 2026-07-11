# 会话管理数据源隔离

## 背景

会话管理页同时显示 Codex 会话管理与 Claude 会话管理。当前 Claude 面板复用了 Codex 的 `localSessions` 数据源，导致两个面板的数据库路径、项目和会话内容完全一致，用户无法判断 Claude 会话是否被真实读取。

## 目标

- Codex 会话面板只显示 Codex 本地 SQLite / rollout 会话。
- Claude 会话面板不得复用 Codex 会话数据。
- Claude 面板必须使用 `spec/claude-session-management.md` 定义的独立真实数据源，不得显示 Codex 内容。
- 刷新历史会话仍刷新 Codex 本地会话与设置，不改变现有删除 Codex 会话能力。

## 非目标

- Claude 会话文件扫描和删除由 `spec/claude-session-management.md` 单独约束；本规格不重复定义其解析细节。
- 本次不删除用户会话数据。
- 本次不改变盘古记忆采集层。

## UI 要求

- Codex 面板显示 Codex 数据库路径、候选库数量、会话数、项目和会话列表。
- Claude 面板显示独立 Claude 会话源、候选源数量、真实会话数和项目列表。
- Claude 面板不得出现 Codex 数据库路径和 Codex 项目列表。

## 技术约束

- 保持 `LocalSessionsResult` 作为 Codex 会话结果。
- 前端渲染函数必须显式接收数据源，不允许闭包隐式读取全局 `sessions/localSessions` 后同时渲染 Codex 与 Claude。
- 添加回归测试，防止 Claude 面板再次调用 Codex 数据源。
