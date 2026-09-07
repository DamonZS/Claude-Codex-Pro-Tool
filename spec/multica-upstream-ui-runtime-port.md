# Multica 原始 UI 与 Codex Runtime 适配移植

## 背景

当前 Codex 注入工作区的 `我的任务`、`自动化`、`智能体` 由
`assets/inject/renderer-inject.js` 以 Shadow DOM 和原生 DOM API 渲染。
虽然其控制面字段模仿了部分 Multica 契约，但它不是上游页面，不能满足
“使用 D:\\Project\\multica 原 UI 和后端处理逻辑”的要求。

本规格定义将上游页面、前端状态语义和业务契约移植到 CCP 的方式。模型
执行、任务/智能体执行、项目会话关联以及 Skill 加载必须只通过当前打开的
Codex 页面 Host 的原生能力完成；不得启动 Multica daemon、CLI、server、
Codex app-server、Claude 或任何第二模型执行器。

## 已核实的上游边界

- `packages/views/my-issues/components/my-issues-page.tsx` 通过
  `IssueSurface` 提供板/列表/表格/泳道、筛选、拖拽与任务视图语义。
- `packages/views/autopilots/components/autopilots-page.tsx` 使用 TanStack
  Query、虚拟列表、筛选/排序/列控制、对话框、触发器和运行记录。
- `packages/views/agents/components/agents-page.tsx` 使用 TanStack Query、
  Zustand、虚拟列表、权限、运行时、状态和 30 天运行统计。
- 上游 `packages/core/api/client.ts` 是完整 HTTP API 客户端，依赖认证、
  workspace 路径、schema 校验、query cache、乐观更新与实时失效；它不能
  直接指向 Codex 页面 Host 或由少量 fetch 替代。
- CCP 已有 `multica_workspace`、`multica_execution` 与 `CodexPageHostTransport`
  控制面。它可以成为本地 API 适配器的权威实现，但当前只覆盖上游契约的一部分。

## 目标

1. 仅在 Codex 左侧原生“插件”之后提供 `我的任务`、`自动化`、`智能体` 三个
   入口；不注入 `项目` 或 `Skill`。Codex 原生项目区域不得改动。
2. 三个入口分别挂载经锁定上游版本移植的 `MyIssuesPage`、`AutopilotsPage`、
   `AgentsPage` 及其实际依赖，而不是重新设计的等价页面。
3. 保留上游页面的布局结构、控件层级、状态、空/加载/失败/无权限处理、筛选、
   排序、虚拟化、拖拽、弹窗与乐观更新语义。容器只继承 Codex 原生背景；不向
   Codex 原生 React 树、标题栏、项目区或模型选择器写入状态。
4. 把上游 API client 的每个被移植页面实际调用的 endpoint 替换为本地、类型化、
   白名单化的 Codex Runtime Adapter。请求和事件映射必须保留上游 DTO、错误码、
   revision/CAS、权限和缓存失效语义。
5. 创建、继续、取消任务和自动化执行时只创建/操作 Codex 原生 task/thread/
   subagent；Skill 清单只由 Codex 原生 inventory 和页面 Host capability 决定。

## 非目标

- 不复制或改写 Codex 原生项目、Skill 管理、供应商、模型选择、账号、配置或 UI。
- 不实现上游收件箱、聊天、云端身份、支付、托管 runtime、daemon、CLI 或服务端。
- 不以 iframe、外部网页、静态截图、假数据或第二执行器代替可运行页面。
- 不把 D:\\Project\\multica 作为运行时路径或发行时隐式依赖。

## 许可和发布门禁

锁定上游的 `docs/third-party/multica/LICENSE` 明确要求：派生自上游 UI 的
发行物必须保留 Multica 的产品名、Logo、版权及归属信息，除非取得生产方的书面
品牌豁免；完整许可证和 NOTICE 必须随派生发行物提供。

本次集成采用第一条路径：保留 Multica 的产品名、Logo、版权和归属，并在发行物中
携带完整 `LICENSE` 与 `NOTICE`。该决定由需求方于 2026-09-04 确认。源版本、文件
哈希、复制范围和后续修改记录必须写入
`docs/third-party/multica/SOURCE_MANIFEST.md`。

如后续面向第三方运营托管服务、销售、许可或以其他商业方式分发嵌入的 Multica UI，
发布负责人仍须按上游许可证第 1(a) 条确认适用的商业许可；保留品牌不替代该独立
要求。该项不允许被代码或构建步骤伪造为已满足。

## 设计

### 可构建移植边界

在仓库内新增一个独立、可审计的前端工作区包，例如
`apps/codex-workflow-surface/`。它只能包含锁定版本上游页面所需的最小闭包：

- 视图：My Issues、Autopilots、Agents；
- 共享：IssueSurface、列表/对话框/表单/导航/i18n 所需模块；
- core：types、queries、mutations、stores、API adapter interface；
- UI：页面实际 import 的组件、样式、图标、字体和测试 fixture。

移植文件必须保留上游版权头、添加 CCP 修改说明，并在 `THIRD_PARTY_NOTICES` 中
记录上游 commit、来源路径和 SHA-256。不得把整个上游仓库或本机
`D:\\Project\\multica` 目录纳入 Git 或发行物。

### Codex Runtime Adapter

新增 `MulticaApiAdapter`，由上游 API client 注入，而非让页面直接调用
`fetch`。适配器只暴露页面实际使用的白名单方法：

| 域 | 读操作 | 写/执行操作 | Codex 映射 |
| --- | --- | --- | --- |
| 我的任务 | issues、views、labels、成员、状态 | create/update/move/assign/comment | 任务状态保存在本地控制面；显式执行创建当前页面的 Codex task/thread |
| 自动化 | autopilots、triggers、runs、quota | CRUD、pause/resume、trigger | 调度和审计保存在控制面；触发只派发当前页面 Host 的 Codex 原生执行 |
| 智能体 | agents、runtime capability、presence、activity、skills | CRUD、权限、Skill 绑定 | 智能体定义/审计保存在控制面；运行、Skill inventory 和 subagent 由 Codex Host 返回 |

所有 mutation 带 `command_id`/幂等键及 `expected_revision`；409、403、超时、断线、
Host capability 缺失必须转换为上游页面可展示的确定错误。适配器不能接受任意 URL、
header、命令、文件路径、环境变量或浏览器传来的权限声明。

### 运行与事件

`CodexPageHostTransport` 是唯一执行 transport。Runtime Adapter 先写入一次
本地 attempt/审计，再向页面 Host 发受限 DTO，保存返回的 thread/task/subagent ID。
重复请求复用相同映射；执行完成、失败和取消经事件游标回写。Host 不可用时保留
可编辑任务、自动化和智能体数据，但停止派发且清晰显示降级，不启动替代 Runtime。

### UI 容器

原页面挂载到单个 Shadow DOM host，host 只负责容器尺寸、Codex 背景继承、
显示/隐藏、deep-link route 和错误边界。不得增加 CCP 顶部壳、永久第二侧栏、
黑色背景、品牌替换、iframe 或覆盖原生项目/对话。

## 分阶段交付

1. **许可和闭包审计**：固定上游 revision，取得品牌/商业分发依据，生成依赖清单、
   source manifest 和 API 方法矩阵。
2. **可构建 My Issues**：移植最小 React 闭包，接入只读 Adapter，再完成 issues/
   views/labels/状态 mutation 和拖拽回滚。
3. **自动化**：移植列表、编辑器、触发器和运行记录；实现 schedule/webhook/API
   的队列、租约、幂等与 Codex Host 派发。
4. **智能体**：移植列表、详情、权限、运行时、统计与 Skill 绑定；实现原生
   Codex inventory、capability 和 subagent 映射。
5. **真实验收与发布**：以默认 `target/release` 最新构建重新注入 Codex，完成 UI
   截图、原生 thread 映射、无第二执行器、配置不变与许可包检查。

## 交付物

- 上游 source manifest、完整 LICENSE/NOTICE、变更说明和上游版本锁定。
- 可构建的移植包及三页真实页面。
- 类型化 Adapter、DTO/schema、存储迁移、事件同步、权限/幂等测试。
- Codex 注入挂载器和端到端 UI 测试。
- 对应验收文档、构建产物、运行/回滚说明。
