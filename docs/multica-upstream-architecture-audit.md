# Multica 上游源码架构审计

## 审计目的

回答“能否直接拿 Multica 源码，改几个接口后放进 Codex”这一问题，并为后续集成选择提供可复核的源码依据。本审计只读取当前 `.upstream-multica` 快照，不修改该目录，也不把上游代码误当作 Codex 原生 API。

审计对象：`multica-ai/multica` 上游仓库（当前工作区快照 `.upstream-multica`）。

## 结论先行

**技术上可以直接复用上游源码；工程上不能只复制页面再补少量接口。**

要做到接近上游“一比一”，必须同时选择下面两条路线之一：

1. **完整复用路线**：把 `packages/core`、`packages/views`、`packages/ui` 连同 pnpm workspace、React Query/Zustand、路由和构建链整体纳入，并运行上游 Go server、PostgreSQL、鉴权、WebSocket 和 daemon。Codex 只作为一个被上游 daemon 调用的 CLI runtime。
2. **控制面适配路线**：只复用上游领域模型、页面行为和接口契约，在 CCP 内重建本地存储和受控 bridge；不启动上游 server/daemon/CLI，也不声称这是上游原生 UI/API。当前 CCP 规格选择的是这条路线。

“把源码拿来改改”只有在完整复用路线下才成立；在控制面适配路线下，直接拷贝上游页面反而会引入无法满足的运行时依赖和错误的执行语义。

## 前端入口和页面关系

上游桌面入口位于 `apps/desktop/src/renderer/src/routes.tsx`。该文件通过 `createMemoryRouter` 注册工作区路由，并从共享包导入页面：

- `@multica/views/my-issues`：我的任务看板
- `@multica/views/issues`：任务列表/看板和 Issue 详情
- `@multica/views/projects`：项目列表和详情
- `@multica/views/autopilots`：自动化及运行历史
- `@multica/views/agents`：智能体列表、创建和详情
- `@multica/views/skills`：Skill 列表和详情
- `@multica/views/squads`：小队
- `@multica/views/dashboard`：统计
- `@multica/views/settings`：设置
- 桌面专属 `DesktopRuntimesPage`、`DesktopAgentsPage`：运行时和桌面智能体能力

这些页面不是自包含 HTML。它们依赖共享 `@multica/core` 类型/API、`@multica/ui` 组件、i18n、React Query/Zustand 状态和工作区上下文。上游路由还是工作区 slug 作用域的内存路由，不等价于 Codex renderer 的 URL/history 路由。

## 构建和依赖边界

根 `package.json` 明确使用 `pnpm@10.28.2`、Node `>=22` 和 Turbo；`pnpm-workspace.yaml` 把 `apps/*`、`packages/*` 纳入同一依赖图。构建、类型检查和测试都是 Turbo 过滤后的跨包任务。

共享依赖包括 React 19、React Router、TanStack Query/Table/Virtual、Zustand、i18next、Zod、Tailwind、Lucide 等。只拷贝某个页面文件会失去其包内导出、Provider、schema 校验和缓存失效逻辑；结果通常是编译通过但运行时缺少 workspace、用户、API client 或 query context。

## API 和数据模型

`packages/core/api/client.ts` 是统一 HTTP 客户端，不是简单的几个 `fetch` 调用。它覆盖 Issue、Project、Agent、Squad、Autopilot、Skill、Runtime、评论、标签、Reaction、Activity、聊天和资源等独立资源，并统一处理工作区 slug、认证头、幂等键、`If-Match`、schema 解析和错误映射。

典型领域类型位于 `packages/core/types/`：

- `issue.ts`：Issue、状态类别、负责人、项目、父子关系、属性、标签、活动摘要等。
- `agent.ts`：Agent、AgentTask、Runtime、Skill 绑定、并发和运行状态。
- `autopilot.ts`：schedule/webhook/API trigger、run/history、暂停和授权字段。
- `project.ts`：项目、成员、日期、资源和进度。
- `workspace.ts`：工作区、成员和权限上下文。

因此，接口层不是“给页面加一个 endpoint”即可替换；必须保持类型、schema、缓存键、乐观更新和错误码的一致性。

## 服务端、权限和状态权威

服务端入口 `server/cmd/server/router.go` 注册 HTTP、插件桥、公开 API、WebSocket 和 daemon 路由。请求通常经过：

1. `server/internal/middleware/auth.go`：Bearer JWT/PAT 或 cookie/CSRF；`mat_` 任务 token 会被绑定到 user、agent、task、workspace。
2. `server/internal/middleware/workspace.go`：解析 workspace slug/UUID，校验成员关系和角色，并拒绝任务 token 跨工作区访问。
3. handler/service/storage：执行字段校验、业务状态变更、数据库事务和实时事件广播。

上游 Issue 移动端点 `server/internal/handler/issue_move.go` 接收 `before_id`、`after_id` 和 `expected_revision`，服务端解析邻居并计算位置，再复用 `UpdateIssue` 的校验、事件和任务触发逻辑。前端不能只改卡片所在列来代替该写入。

所以直接复用上游 API 至少需要数据库 schema、迁移、鉴权密钥、workspace 成员、事件总线和错误协议；把请求改成本地 JSON 文件并不等价。

## 任务执行链

上游执行链位于 daemon 侧，而不是 Codex App renderer：

`claim task -> prepare workspace/worktree -> start task -> execute backend -> report progress/messages/usage/result`。

关键实现包括：

- `server/internal/daemon/client.go`：Claim、Start、ReportProgress、ReportTaskMessages、ReportTaskUsage 等 daemon API。
- `server/internal/service/task.go`：任务领取、状态迁移、租约和报告。
- `server/internal/daemon/daemon.go`：运行时探测、任务执行和结果回写。
- `apps/desktop/src/main/daemon-manager.ts`：CLI/daemon 安装、profile、健康检查、恢复和生命周期管理。

daemon 最终启动的是本机 Codex CLI（上游把 Codex 当作 runtime），不是 Codex Desktop 当前 renderer 暴露的 `thread/create`、`subagent` 或 `agent/list` API。当前 CCP 没有证据证明存在可直接调用的完整原生 API，因此不能把上游 daemon 链复制进 CCP 后宣称“调用 Codex 原生会话”。

## 可直接复用与不可直接复用

### 可以直接复用

- 领域类型和字段语义；
- Issue 七列、拖拽邻居和 revision/CAS 规则；
- Agent/Squad/Autopilot/Skill 的业务状态和权限语义；
- `client.ts` 中的请求路径、请求/响应 schema 和错误码（前提是同时提供对应服务端）；
- 共享 UI 的视觉和交互实现（前提是迁移完整依赖、Provider、i18n 和样式包）。

### 不能只复制后“补接口”

- 不能在没有上游 workspace Provider、QueryClient、i18n 和 UI 包的情况下单独复制页面；
- 不能在没有 Go server、PostgreSQL、迁移和 auth/workspace middleware 的情况下直接使用上游 API client；
- 不能把上游 daemon/CLI 执行链当成 Codex renderer 原生执行；
- 不能用本地缓存或页面状态冒充 thread、subagent、Skill 实际加载完成；
- 不能移除上游许可证、NOTICE、归属和修改声明来“清除标识”。

## 对 CCP 的落地建议

当前 CCP 若要保持“单一 Codex 入口、默认不启动第二套 runtime”的约束，应继续采用控制面适配路线：

1. 从上游源码提取类型/契约和可验证的 UI 行为，不复制需要 Electron/Go/DB 的运行时模块。
2. 在 CCP bridge 中提供窄接口：Issue/Project/Agent/Squad/Autopilot/Skill 查询与 CAS 写入，以及 Codex host 能力探测。
3. Codex 执行只接受当前页面真实探测到的 host 能力；不可用时返回 `unsupported`/排队，不走 CLI、daemon 或第二个 app-server。
4. 每个执行映射持久化 Multica run/attempt、Codex thread/task ID、revision、幂等键和最后事件；完成状态必须来自 Codex 真实事件。
5. 若产品目标改为真正复用上游页面，则应另开完整复用项目：引入上游 pnpm workspace 和构建产物，部署 Go server/PostgreSQL/WebSocket，明确 daemon/CLI 作为执行器，并重新评估 Codex 集成与许可证边界。

## 许可证与归属

上游审计快照仅用于本地分析，位于未跟踪的 `.upstream-multica/`，不作为 CCP 的运行时依赖或发布内容。仓库中 `docs/third-party/multica/` 当前仅保存 `LICENSE` 与 `NOTICE`；任何后续复制或衍生上游源码的发布都必须同时纳入完整对应源码、许可证、NOTICE、版权和修改声明。上游 revision 记录必须在后续提交中统一，避免源码审计版本与归属声明不一致。

## 可复核命令

```powershell
Get-Content -Encoding UTF8 .upstream-multica/apps/desktop/src/renderer/src/routes.tsx
Get-Content -Encoding UTF8 .upstream-multica/packages/core/api/client.ts
Get-Content -Encoding UTF8 .upstream-multica/server/cmd/server/router.go
Get-Content -Encoding UTF8 .upstream-multica/server/internal/middleware/auth.go
Get-Content -Encoding UTF8 .upstream-multica/server/internal/middleware/workspace.go
Get-Content -Encoding UTF8 .upstream-multica/server/internal/handler/issue_move.go
Get-Content -Encoding UTF8 .upstream-multica/server/internal/daemon/daemon.go
Get-Content -Encoding UTF8 .upstream-multica/apps/desktop/src/main/daemon-manager.ts
```
