# Multica 上游源码对接映射

本文记录当前对照的上游仓库版本：`multica-ai/multica`，提交 `2e297451001e65f78721efc24e36e7939e0f0ed6`。

## 上游真实职责

上游不是 Codex App 的原生任务实现，而是一个独立的 Web/Desktop/Server/CLI + agent daemon 系统：

- `packages/core/types/agent.ts`：Agent、AgentTask、runtime、Skill 摘要等领域类型。
- `packages/core/api/client.ts`：创建 Agent、任务、运行时和 Skill 的 HTTP API 客户端。
- `packages/views/agents/`：Agent 列表、详情、创建、runtime/Skill 选择和权限 UI。
- `packages/views/issues/`：Issue 看板、分配、执行状态和 review 流程。
- `server/` 与 `cmd/`：服务端 API、任务编排和 CLI 入口。
- `apps/desktop/src/main/daemon-manager.ts` 及 `CLI_AND_DAEMON.md`：本机 daemon 生命周期、CLI 探测、任务领取和子进程执行。

上游的 Codex 支持属于可选 CLI runtime，典型流程是：daemon 注册 `codex` runtime，领取 Issue，创建隔离工作目录，再启动 Codex CLI。上游没有可直接调用的 Codex App renderer `agent/list` 或 `thread/fork` 协议。

## 本项目的受控映射

本项目将上游领域能力落到嵌入式 Multica 控制面，但执行权保持在当前已打开的 Codex 页面：

| 上游能力 | 本地实现 | 权威来源 |
| --- | --- | --- |
| Issue/项目/Project Resource/Agent/Squad/Autopilot | `multica_workspace.rs` 的本地持久化集合、独立 `project_resources` 资源和看板操作 | 本地 workspace store |
| Agent -> runtime 执行映射 | `multica_execution_store.rs` 的 attempt/binding/幂等记录 | Codex 页面 host 返回的真实句柄 |
| Codex 执行 | `codex_execution.rs` 的 allowlist typed adapter | 当前 Codex renderer 的 page host |
| Skill 清单与加载 | `skills/list` + trust/digest 校验 | Codex page host + 本地信任快照 |
| 原生项目/会话显示 | `renderer-inject.js` 读取当前 DOM 的稳定原生数据属性并触发行点击；bootstrap 同时投影本机 SQLite 的 `threads`、`project_roots`、`thread_spawn_edges`、`thread_dynamic_tools` 只读快照 | 当前 Codex 页面 + `~/.codex` SQLite |
| daemon/CLI/runtime 注册 | 不接入默认路径 | 不适用；禁止启动第二套 runtime |

## 明确限制

原生 Codex 状态库审计还确认：项目与会话的归属需要使用 `project_roots.path` 与线程 `cwd` 匹配，不能只依赖 `threads.project_id`；历史续接失败的直接错误为 HTTP continuation 缺少 `call_id`，而 `previous_response_id` 仅支持 Responses WebSocket v2。该错误属于宿主续接链路，不得由工作区伪造成功。

当前 Codex host 未暴露 `agent/list` 时，Agent 列表只能显示本地 Multica Agent 及其“已绑定/未绑定原生执行”状态，不能伪造 Codex Agent 清单。当前页面没有原生项目或会话行时，界面显示空态；不会从缓存推断实时存在。

验证上游源码时使用的命令：

```powershell
git clone --depth 1 https://github.com/multica-ai/multica.git
rg -n "createAgent|assign.*issue|daemon-manager|thread/fork|agent/list" .
```

## 本轮源码与文档核对记录

本轮以上述锁定提交为准，实际阅读了类型、客户端、视图、服务端 handler、daemon 实现与运行文档，而非只读取 README：

| 上游证据 | 核对结论 | 本地边界 |
| --- | --- | --- |
| `packages/core/types/issue.ts` | Issue 除基本字段外还包含 `status_category`、`status_name`、`metadata`、`properties`、`labels`、`reactions`、`last_activity_at`、`source_context` | 本地编辑器接入安全 JSON 字段；标签/反应/评论仍只读摘要或明确未接线 |
| `packages/core/types/comment.ts`、`server/internal/handler/comment.go` | 评论是独立线程资源，含 parent、resolve、reaction、attachment 和 mention trigger 结果 | 本地已接入独立 `comments` 集合与 CAS CRUD；附件、通知、mention 调度仍未接线 |
| `packages/core/types/label.ts`、`server/internal/handler/label.go` | 标签是独立资源，限定 issue/agent/skill 类型并规范颜色 | 本地已接入独立 `labels` 集合与颜色/类型校验；远端同步和关联端点仍未接线 |
| `packages/core/types/comment.ts`、`server/internal/handler/reaction.go` | 评论反应独立保存 `comment_id`、actor 和 emoji，并通过事件广播 | 本地已接入独立 `reactions` 集合与输入校验；事件广播和远端关联端点仍未接线 |
| `packages/core/types/activity.ts`、`server/internal/handler/activity.go` | Activity 与 Comment 合并形成 Issue timeline，支持操作者、动作、详情和时间分页 | 本地已接入只读 `activities` 集合投影；远端分页、实时事件和服务端生成仍未接线 |
| `packages/core/api/client.ts` | 存在 comments、issue reactions、subscribers、labels、project resources、activity 等独立客户端方法 | 后续按独立资源逐项接入，当前 bridge 不透传任意 URL/method |
| `packages/core/types/agent.ts`、`server/internal/handler/agent.go` | Agent 有 runtime、permission、invocation targets、并发、模型/思考级别、Skill 和 activity 等字段 | 本地编辑器接入非敏感业务字段；MCP/custom env 等敏感配置不进入 renderer |
| `packages/core/types/project.ts`、`server/internal/handler/project_resource.go` | Project 资源是独立 `ProjectResource`，类型为 `github_repo` / `local_directory`，另有成员、日期和进度语义 | 本地已增加独立 `project_resources` 集合和 CAS CRUD；daemon 目录锁/worktree 仍不由页面桥接伪造 |
| `packages/core/types/autopilot.ts`、`server/internal/handler/autopilot.go` | Autopilot 支持 schedule/webhook/API trigger、启停和 run/history | 本地仅维护安全调度描述和执行映射，未启动第二执行器 |
| `server/internal/daemon/daemon.go`、`apps/desktop/src/main/daemon-manager.ts` | 上游实际通过 daemon claim/runTask 后启动 Codex CLI；不是 Codex App renderer API | CCP 明确不复制 daemon/CLI/app-server，执行只走当前 Codex page host |
| `CLI_AND_DAEMON.md`、`VISION*.md`、`SELF_HOSTING*.md` | 上游部署/运行模型是独立 Web/Desktop/Server/CLI 系统 | 这些文档用于解释能力来源，不作为当前 Codex 原生能力证明 |

因此，当前“忠实对接”的判定按能力证据拆分：上游领域模型和安全字段可以投影到本地工作区；只有当前 Codex 页面实际探测到的 host 方法才能执行，未暴露的 subagent、评论写入、标签 CRUD 等能力必须显示未接线/不支持，不能用本地缓存冒充成功。
