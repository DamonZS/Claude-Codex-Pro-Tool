# 验收标准：Claude 会话管理真实数据链路

验证对象：`spec/claude-session-management.md`

## 验收项

1. 独立 Claude 数据源
   - 通过标准：存在 `list_claude_sessions`，读取 Claude JSON/JSONL 来源；Claude 面板使用独立 `claudeSessions`，不复用 `localSessions`。
   - 证据：Core/Tauri 单元测试与前端源码回归测试。

2. 本机真实会话可见
   - 通过标准：当 `~/.claude/projects/<project>/*.jsonl` 存在有效 user/assistant 记录时，返回会话数大于 0，项目、标题和更新时间可用。
   - 证据：fixture 测试及本机只读命令结果（只报告数量和元数据，不输出正文）。

3. 多格式与容错
   - 通过标准：逐行解析 JSONL；支持字符串用户内容和数组 assistant 内容；识别 audit/local 直接来源；单个畸形行不阻断同文件有效会话；嵌套 `subagents` 文件不重复成为顶层会话。
   - 证据：Core 定向测试。

4. 刷新反馈与真实统计
   - 通过标准：Claude 面板有可点击刷新按钮，刷新调用 `list_claude_sessions` 并更新候选源数、会话数和通知；页面不再出现“待接入/扫描尚未接入”。
   - 证据：前端类型检查、UI 源码回归测试和手动检查。

5. 点击查看真实上下文
   - 通过标准：点击 Claude 会话行调用 `load_claude_session_context`，浮框展示按角色区分的真实消息；初次只加载最近一页，支持加载更早内容；关闭交互可用。
   - 证据：Core 分页解析测试、Tauri/前端接线回归测试和手动点击截图。

6. 备份后删除
   - 通过标准：`delete_claude_session` 只允许删除重新发现的 Claude 会话源；成功时源文件消失且备份文件内容一致；备份失败或路径不受信任时源文件保留。
   - 证据：Core 删除测试和 Tauri 命令接线测试。

7. 安全边界
   - 通过标准：列表结果和日志不包含完整会话正文、工具参数、API key、Bearer token、cookie；不修改盘古记忆数据库、Claude 中文注入或 Codex 会话数据库。
   - 证据：diff 审查和日志字段检查。

8. 构建与回归
   - 通过标准：相关 Rust 测试、Manager UI 回归测试、前端检查/构建、`cargo fmt --check` 和 Manager debug 构建通过。
   - 证据：命令输出。

## 必需验证

```powershell
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml claude_sessions -- --nocapture
cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem session_management_route_contains_history_and_codex_claude_session_management -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo fmt --check
cargo build -p claude-codex-pro-manager --manifest-path Cargo.toml
```

## 完成证据

- 新命令已注册且前端真实调用。
- fixture 首次扫描返回 Claude 会话。
- 本机存在 Claude 会话源时，真实扫描结果不再为 0。
- 删除 fixture 会话后备份存在且源文件不存在。
- 不再展示占位状态。

## 不在范围内

- Claude 官方网页缓存会话恢复。
- 会话正文编辑、批量删除、云端同步。
- 盘古记忆和 Claude 中文注入行为变更。
