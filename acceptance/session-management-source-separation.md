# 验收标准：会话管理数据源隔离

验证对象：`spec/session-management-source-separation.md`

## 验收项

1. Codex / Claude 数据源隔离
   - 通过标准：Codex 面板使用 `localSessions`；Claude 面板不把 `localSessions` 传入会话浏览器。
   - 证据：前端源码断言。

2. Claude 不再显示 Codex 会话
   - 通过标准：Claude 面板显示独立待接入状态或独立 Claude 数据源，不显示 Codex 数据库路径和 Codex 会话列表。
   - 证据：源码断言和手动截图。

3. 现有 Codex 会话管理不回归
   - 通过标准：Codex 面板仍可显示数据库、候选库、会话数、项目列表和删除按钮。
   - 证据：前端检查和回归测试。

4. 构建验证
   - 通过标准：TypeScript 检查、前端构建、相关 Rust 测试和 manager 构建通过。
