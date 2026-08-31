# 验收标准：Codex 内嵌 Multica 工作区集成

## 对应规格

本验收标准验证：

- `spec/codex-multica-workspace-integration.md`

并继承以下文档中不被本规格扩展的隔离要求：

- `acceptance/multica-runtime-adapter.md`（仅在用户显式启用外部连接兼容能力时适用）
- `acceptance/multica-managed-runtime.md`（仅在用户显式进入并启用 legacy 高级兼容能力时适用；不属于本地工作区默认启动验收）
- `acceptance/codex-injection-content-boundary.md`
- `acceptance/remove-codex-model-selection-injection.md`

## 验收原则

- 开发阶段只运行能证明当前代码改动及其直接影响范围的最窄测试。
- 每个增量通过后保留命令、退出码和关键断言，不用反复全量构建代替定向证据。
- 只有最后一轮发布验收运行完整前端构建、workspace 测试、release 构建和真实 Codex 大范围回归。
- 静态源码断言不能替代真实运行验证；涉及入口位置、重注入、窗口几何、原生 thread 和进程行为的项目必须提供实时 CDP/Playwright 或进程证据。
- 验收使用最新默认 `target/release` 构建。旧 Codex renderer、旧注入脚本或临时 `target-rebuild` 产物不能作为最终证据。
- 任一关键项只有“计划”“代码存在”或“请求返回 2xx”而没有权威状态证据时，视为未通过。

### 本地模式硬性边界

- 被测形态是随 CCP 运行的本地 Multica 控制面/编排层，不要求安装或运行完整 Multica Web、Desktop、server 或独立执行服务。
- 验收过程中不得调用 `register_managed_codex_runtime` 或等价 API，不得创建 `CodexAppServerTransport`/JSON-RPC app-server 通道，不得由 Multica daemon/worker 启动、托管或连接 `codex.exe app-server`。
- 任务执行、Skills 加载、thread/subagent 状态和事件必须来自当前 Codex 页面/renderer 的原生 host API；能力不可用时只能返回 `unsupported` 或排队，不能改走 CLI、shell、HTTP 代理、第二窗口或伪造结果。
- 文档或界面中的“运行时”只可表示控制面/当前页面能力状态，不得暗示存在需要注册的新 Codex runtime。
- 默认启动必须是纯本地工作区路径：不得下载/安装 Multica CLI、创建托管 profile、发起登录、启动/监管 daemon，且 Manager 主导航不得出现独立 `Multica Runtime` 页面。

## 测试基线

最终端到端验收前准备：

- 一个干净或可重置的 Multica 测试 workspace。
- 一个已登录、支持被测原生 task/thread 能力的 Codex App。
- 本地 Multica 控制面的初始化成功、数据不可用、可选同步失败三类 fixture，以及 Codex 页面原生能力可用/不可用 fixture；不创建 Codex app-server 进程 fixture。
- 可控制成功、401/403、409 revision、超时、断线、事件 gap 和重复响应的 fake Multica server。
- 可控制创建、打开、继续、取消、完成、失败、丢失和重复事件的 fake `CodexExecutionService`。
- Windows 至少一个常用桌面窗口和一个窄窗口；macOS 发布目标启用时提供等价窗口证据。
- 验收前记录 CCP 供应商/Profile、代理状态、Codex 模型菜单、关键 Codex/Claude 配置文件摘要以及相关进程基线，凭据必须脱敏。

## 验收项

### 1. 范围与架构边界

#### AC-01：双权威边界明确

通过标准：

- Multica 是 workspace、Issue、项目、分配、自动化、审计和调度状态权威。
- Codex 是原生 task/thread/subagent 及其真实执行状态权威。
- CCP 只保存映射、幂等、游标和对账状态，不实现第二套模型执行器；生产派发只调用当前 Codex 页面原生 host API，不注册或启动任何新的 Codex Runtime。

验证方式与证据：

- 架构/接口测试展示 `MulticaClient` 与 `CodexExecutionService` 的独立边界。
- 一份端到端事件记录同时包含 Multica run ID、Codex thread ID、attempt、revision 和 correlation ID。
- 代码评审确认注入页面不直接调用远端 Multica 服务，且本地 Multica 控制面、任何可选同步 worker 都不直接调用供应商或模型 API。

#### AC-02：现有规格边界未被弱化

通过标准：本实现没有恢复 Codex 模型选择注入、Codex 通用文本翻译、供应商 URL 改写、代理自动切换或 Claude/Codex 配置覆盖。

验证方式与证据：相关既有验收测试仍通过，并提供集成前后关键配置摘要差异为零的记录。

### 2. 默认启用、自动准备与降级

#### AC-03：入口默认启用且启动不阻塞

通过标准：

- 新安装缺少设置字段时，`multica_workspace_enabled`（或等价稳定字段）默认为 `true`；升级和重启保留用户已保存的 `true/false`，不回填覆盖。
- 开关为 `true` 时显示工作区入口并初始化本地控制面；开关为 `false` 时不创建入口、不挂载页面壳、不启动本地轮询或新派发。
- 两种开关状态下都不得下载/安装 Multica CLI、创建托管 profile、登录、启动/监管 daemon、注册 Codex runtime 或启动 `codex.exe app-server`。
- Codex 原生界面先正常可用，本地控制面准备异步进行。
- 本地数据不可用时显示真实错误/重试状态；可选同步未登录时显示 `needs_login`；控制面 ready 时加载工作区。

验证方式与证据：使用干净设置、保存为关闭、保存为开启三类 fixture，分别执行启动、重启、刷新和重新注入；记录持久化值、入口/页面壳节点数、轮询与派发调用计数、UI 截图和 Codex 输入可交互证据。进程/网络/文件写入 spy 必须证明默认路径零 CLI 下载、零 profile 创建、零登录、零 daemon、零 runtime 注册和零新增 Codex app-server 进程。

#### AC-04：本地控制面故障只降级工作区

通过标准：本地控制面初始化失败或可选同步健康检查失败时，工作区显示脱敏错误和恢复动作；Codex 对话、CCP 代理、供应商和 Claude 启动仍可用，且不尝试下载 legacy 资源、启动 daemon 或启动 Codex app-server。

验证方式与证据：注入控制面/同步失败后运行最窄生命周期测试，并在真实 Codex 中证明原生导航和输入未被阻塞、进程树无新增 Codex 执行器。

### 3. Codex 左侧入口

#### AC-05：入口位于原生“插件”下方、项目上方

通过标准：

- 入口使用原生插件按钮作为锚点。
- `Multica 工作区` 与插件同属一个导航父节点。
- DOM 顺序和视觉顺序均为插件在前、Multica 在中、项目在后。
- 中英文 Codex 文案均可定位。

验证方式与证据：

- 注入契约测试覆盖 `pluginEntryButton()`、`selectors.pluginNavButton`、`插件|Plugins` fallback、`insertBefore(..., pluginEntry.nextSibling)` 和唯一入口约束。
- 实时 CDP 返回插件、Multica 和项目三个节点的共同父节点、bounding box 和 DOM order。
- Windows 截图清楚显示入口位于插件下方、项目上方且无重叠。

#### AC-06：重注入保持单实例

通过标准：初次注入、连续注入三次、切换原生页面、侧栏折叠/展开和 Codex React 重绘后，`[data-ccp-multica-nav]` 与工作区根节点各不超过一个，事件只触发一次。

验证方式与证据：Playwright/CDP 自动执行上述动作，记录每一步节点数量、一次点击对应的 bridge 请求数和最终截图。

#### AC-07：锚点缺失时不破坏原生导航

通过标准：插件锚点暂时不存在时只进行有界重试；不会插到会话列表、标题栏、输入区或随机按钮附近，不产生无限 observer/定时器。

验证方式与证据：无锚点 fixture 的定向测试记录重试上限、诊断事件和零错误插入。

### 4. 本地页面壳与导航

#### AC-08：无 iframe、无外部页面依赖

通过标准：

- 工作区页面壳由 CCP 本地资源渲染。
- 页面壳内 `iframe` 数量为零。
- 打开十个模块不加载 Multica Web、CDN 或远程前端 bundle，不需要外部浏览器完成工作流。
- 页面与 Multica 的业务通信只经过受控 CDP bridge。

验证方式与证据：CDP DOM 查询、Network 记录和资源来源清单；记录所有业务请求均为本地 binding 调用。许可归属页上用户主动打开上游仓库的链接不视为核心流程依赖。

#### AC-09：页面壳不接管 Codex 原生状态

通过标准：工作区只占主内容区域，保留原生侧栏和窗口控件；点击任意原生导航项可恢复 Codex 内容；打开/关闭不会修改 Codex URL、history、原生 React store 或项目选择。

验证方式与证据：实时记录打开前、打开中、关闭后的 URL/history、原生导航选择、输入草稿和 DOM 节点身份，证明原生内容未被删除重建。

#### AC-10：十个模块均为真实页面

通过标准：按以下顺序提供并可进入：

1. `我的任务` / `my-issues`
2. `任务` / `issues`
3. `项目` / `projects`
4. `自动化` / `autopilots`
5. `智能体` / `agents`
6. `小队` / `squads`
7. `统计` / `usage`
8. `运行时` / `runtimes`
9. `Skills` / `skills`
10. `设置` / `settings`

每个模块至少能读取真实数据，并具备加载、空、错误、过期和无权限状态，不允许只有外链、静态说明或假数据占位。

验证方式与证据：Playwright 逐项点击，断言稳定路由键、标题、真实查询操作和五类状态 fixture；输出一组模块截图和 bridge 调用摘要。

#### AC-11：响应式、键盘与样式隔离

通过标准：

- 常用桌面和窄窗口下无文字/控件重叠、横向溢出或标题栏遮挡。
- 入口、内部导航、对话框、表单和看板可以键盘访问，focus 清晰。
- 状态不只靠颜色表达，并尊重 reduced motion。
- 工作区样式不改变 Codex 输入框、模型菜单、插件页、主题或窗口控制按钮。

验证方式与证据：至少两种窗口尺寸的 Playwright 截图、自动几何断言、键盘路径记录和工作区开关前后的 Codex 关键元素 computed style 对比。

### 5. Issue、项目与看板

#### AC-12：Issue CRUD 和字段完整

通过标准：可以创建、读取、编辑和取消/归档 Issue；支持标题、描述、优先级、agent/squad/member、项目、父任务、日期、位置和自定义状态。标题为空、无权限和服务端校验失败均显示字段级错误且不产生脏记录。

验证方式与证据：fake server 契约测试和一个真实 workspace CRUD 回放，包含创建前后实体 JSON 摘要、revision 和 UI 结果。

#### AC-13：项目闭环

通过标准：可以创建/编辑项目、设置 `planned/in_progress/paused/completed/cancelled` 状态、管理日期/成员、把 Issue 归入项目并显示由真实 Issue 计算的进度。

验证方式与证据：定向 API/UI 测试及项目详情截图；进度计算 fixture 覆盖空项目和混合状态项目。

#### AC-14：看板状态与拖拽 CAS

通过标准：

- 看板支持 `backlog/todo/in_progress/in_review/done/blocked/cancelled`。
- 成功拖拽更新一次 revision。
- 409/revision 冲突时撤销乐观移动、刷新卡片并提示冲突，不覆盖对方更新。
- 单纯拖拽不创建 Codex thread。

验证方式与证据：拖拽成功、权限失败、409 和断线四个 Playwright/contract 用例；记录 thread 创建 fake 的调用次数始终为零。

#### AC-15：业务状态与执行状态分离

通过标准：Issue 看板状态和 AgentTask 执行状态分别展示；`dispatched` 不等于 `done`，人工状态变化不伪造 Codex 完成事件。

验证方式与证据：状态映射单元测试覆盖所有组合，并有“已派发但仍 todo/in_progress”“Codex 完成待 Multica 同步”截图。

### 6. 原生 Codex 执行闭环

#### AC-16：唯一原生 thread 创建

通过标准：

1. 未分配 Issue 不创建 thread。
2. 分配可执行智能体或点击执行后，先创建一个 Multica run reservation。
3. 随后只调用一次 Codex 原生 `create_thread`。
4. 成功后持久化 Multica run、Issue、Codex thread/task、attempt 和幂等键映射。
5. Multica UI 显示真实 `dispatched/running`，不提前显示 completed。

验证方式与证据：fake 两侧的调用顺序测试、临时数据库映射断言，以及真实 Codex 中从 Issue 到唯一 thread 的端到端视频/截图和事件日志。

#### AC-17：打开复用同一 thread

通过标准：连续点击“打开对话”三次均聚焦同一个 `codex_thread_id`，不创建新 thread、不发送额外 prompt、不改 attempt。

验证方式与证据：原生导航事件、当前 thread ID、create 调用计数和 UI 截图。

#### AC-18：继续执行复用同一非终态 thread

通过标准：用户确认后，“继续执行”调用原生 continue 能力，使用同一 thread ID 和新的命令幂等键；重复提交只产生一次后续执行/消息，并记录 Multica 审计事件。

验证方式与证据：双击/网络重试 fixture 的调用计数、审计事件和真实 Codex 对话内容摘要。

#### AC-19：终态重跑创建新 attempt 并保留 lineage

通过标准：completed/failed/cancelled thread 重跑时创建新的 `attempt_no` 和 thread，保存 `parent_thread_id/parent_attempt_id`，旧 attempt、状态、日志和映射保持只读可见。

验证方式与证据：临时数据库和 UI attempt 时间线断言；旧、新 thread 均可分别打开。

#### AC-20：原生 subagent 小队映射

通过标准：小队任务创建一个负责人 thread，成员任务只通过 Codex 原生 subagent 创建；每个成员映射含父 thread、成员、attempt 和幂等键。重复派发不会产生重复 subagent，成员失败不会覆盖其他成员状态。

验证方式与证据：fake 原生 subagent 契约测试和一个真实小队用例的父子 ID/状态图。被测 Codex 不支持 subagent 时应通过 `unsupported` 降级用例，不得模拟实现。

#### AC-21：当前 Codex 页面是唯一执行入口

通过标准：执行 Issue、自动化和小队任务期间，只调用当前 Codex 页面/renderer 已暴露且经能力探测确认的原生 task/thread/subagent/Skills host API。不得调用 `register_managed_codex_runtime` 或等价 API，不得启动、托管或连接 `codex.exe app-server`，不得出现任何新增 Codex 执行子进程、npm `codex.cmd`、Claude CLI、其他 Provider、CCP shell 派发或第二执行器。

验证方式与证据：

- 代码搜索/评审覆盖执行适配器调用链，确认没有 runtime registry、app-server transport 或 daemon 注入。
- 端到端进程树快照证明执行期间没有新增 Codex/app-server/npm/Claude/其他 Provider/第二执行器进程；执行发生在当前 Codex 页面进程内。
- CDP/host 事件记录证明模型执行流来自当前页面原生 API；本地 Multica 只接收/写入协调和状态数据。

### 7. 状态、幂等与故障恢复

#### AC-22：AgentTask 状态机完整

通过标准：覆盖 `queued/dispatched/waiting_local_directory/running/completed/failed/cancelled`，非法跳转被拒绝；取消未确认显示 pending；Codex 完成而 Multica 回写失败显示“已完成，待同步”。

验证方式与证据：表驱动状态机测试覆盖每个允许/禁止转换及来源事件。

#### AC-23：mutation 与执行命令幂等

通过标准：同一 `command_id/idempotency_key` 并发或串行发送至少三次，只产生一次 Multica mutation、一次 run 和一个 Codex 执行对象；响应均返回同一稳定 ID。

验证方式与证据：并发测试记录调用计数、唯一索引和返回 ID。

#### AC-24：三类部分失败均可恢复

必须分别通过：

1. Multica 已预留、Codex 创建失败：run 明确失败/可重试，无 running 假状态。
2. Codex 已创建、映射提交失败：恢复后通过幂等键找到原 thread 并完成绑定，不再创建。
3. Codex 已完成、Multica 回写失败：本地保留完成事件，重连后 CAS 回写一次。

验证方式与证据：每个故障点的故障注入测试、重启恢复和最终权威状态快照。

#### AC-25：断线、事件 gap 与游标恢复

通过标准：断线后保留带 `stale` 的只读快照；重连从最后确认游标继续。重复事件被去重，游标过期/gap 触发一次有界对账，不无限拉取历史或重复执行。

验证方式与证据：事件流 fixture 记录请求游标、处理事件 ID、对账次数和最终实体 revision。

#### AC-26：orphaned thread 不被静默替换

通过标准：绑定 thread 不存在时显示 orphaned；打开/继续被阻止，用户只能选择对账、显式重新绑定或新 attempt。任何恢复动作都不删除旧审计。

验证方式与证据：缺失 thread fixture 的 UI、错误码、create 调用计数和恢复后映射。

### 8. Bridge 安全与数据保护

#### AC-27：bridge 仅接受枚举操作

通过标准：

- 只注册规格定义的 workspace/query/mutate/execution/events/reconcile 路径。
- 任意 URL、method、header、Authorization、shell、PID、环境变量和文件路径透传均被拒绝。
- 未知 resource/action、超大 payload、超限分页和过快轮询返回稳定错误码。

验证方式与证据：`bridge_routes` 表驱动测试含成功白名单及上述拒绝用例；路由覆盖清单更新。

#### AC-28：权限与租户边界由 Core 复核

通过标准：伪造 workspace ID、跨 workspace entity ID、过期 revision 和 renderer 自报管理员均不能读取或修改他人数据，403/409 不泄漏实体内容。

验证方式与证据：双 workspace fixture 的跨租户读写测试和脱敏错误快照。

#### AC-29：不可信内容无法执行

通过标准：Issue 标题/描述、Markdown、错误文本和 Skill 元数据中的 HTML、脚本、事件属性、命令文本和 prompt injection 只作为数据展示，不执行 DOM、shell、MCP 或 bridge 动作。

验证方式与证据：XSS/HTML fixture 的 DOM 断言、零意外 bridge 调用和零新增进程证据。

#### AC-30：凭据和正文不泄漏

通过标准：Multica token、Authorization、Cookie、API Key、完整 prompt、完整会话正文不出现在 DOM 属性、local/session storage、URL、普通日志、bridge 诊断或截图。UI 只显示凭据是否已配置。

验证方式与证据：使用哨兵 secret 运行定向测试后，在限定的应用日志、DOM snapshot 和测试数据库中搜索；报告只记录未命中的哨兵摘要，不输出 secret 原文。

### 9. 其余模块闭环

#### AC-31：智能体和 Codex Skills

通过标准：智能体可以创建/编辑/启停、设置能力、并发和受信任 Skills；Skills 页面展示当前 Codex 页面原生 host 的真实 inventory、来源、版本/摘要、信任状态、兼容能力和最近加载结果。单次任务和智能体均可绑定 Skill，派发前形成最终不可变 Skill 清单并通过 `agent-skill-v1` 或 `skill-bundles-v1` 交给同一页面 host。attempt 审计同时记录请求清单和页面原生 API 返回的实际加载结果。绑定未知、未安装、未受信任、冲突或页面能力不支持的 Skill 会阻止派发并进入现有审查流程，不静默忽略，不自动下载安装、执行 hook 或扩大 MCP 权限。

验证方式与证据：智能体 CRUD、任务/智能体 Skill 绑定、最终清单解析、`agent-skill-v1`/`skill-bundles-v1` 能力门禁和页面 host request DTO 测试；真实 attempt 的请求清单与实际加载清单对照；进程/文件系统证明未知 Skill 未被运行或写入，且没有 runtime 注册/新增 Codex 进程。

#### AC-32：小队协作

通过标准：可以维护负责人、成员和分工；UI 显示负责人/成员执行和汇总状态；取消、成员失败、负责人失败均有独立规则和审计。

验证方式与证据：小队 CRUD、部分失败和汇总状态测试及 UI 状态图。

#### AC-33：自动化只调度 Codex 原生执行

通过标准：schedule/webhook/api、启停、手动运行和历史可用；CCP 或当前 Codex 页面离线时 run 排队，不启动第二模型执行器；页面可用后只派发到 AC-21 约束的当前原生 host；租约重领前检查映射，重复触发只执行一次。

验证方式与证据：定时、webhook/API 签名失败、离线、租约过期和重复触发 fixture；记录唯一 run/thread、页面 host 调用和零新增 Codex/app-server 进程。

#### AC-34：统计来自审计数据

通过标准：统计页展示 Issue、attempt、成功率、耗时、队列和 executor 维度，数字来自真实审计/实体，不由前端猜测；空数据和时间范围正确。

验证方式与证据：固定审计数据集的后端聚合测试和 UI 数字比对。

#### AC-35：运行时与设置不复制 Manager

通过标准：Codex 内的运行时页只读显示本地 Multica 控制面和当前 Codex 页面原生能力，不提供 Codex runtime 注册/启动/托管操作。Codex 工作区设置和 Manager 普通设置均操作同一个持久化工作区开关；Manager 主导航不存在独立 `Multica Runtime` 页面。可选外部连接、下载、登录和 daemon 监管仅能从明确标记且默认关闭的 legacy 高级兼容入口进入，敏感配置不存在双份来源。

验证方式与证据：模块 UI、Manager 路由清单、bridge 路由清单、单一设置来源测试和高级兼容入口可达性测试；断言主导航没有 Runtime 项，普通设置仍有可键盘操作的启用/停用开关。

### 10. 隔离回归

#### AC-36：供应商与代理零改写

通过标准：打开工作区、创建/分配/执行/重试 Issue、启动自动化和切换模块均不会：

- 修改任何供应商名称、Profile、Base URL、API Key 或 active profile；
- 把上游 URL 改为 `http://127.0.0.1:57321/v1`、`http://127.0.0.1:57331/v1` 或其他 CCP 代理地址；
- 启停、重配或绕过现有 CCP 代理。

验证方式与证据：Core spy 测试断言相关设置/路由函数零调用；真实端到端前后供应商和代理状态摘要一致。

#### AC-37：Codex 模型选择零注入

通过标准：工作区不创建、修改或监听 Codex 模型选项，不改变当前模型字段，不显示 CCP 模型增强组。

验证方式与证据：既有模型移除测试通过；实时 Codex 中记录工作区开关前后原生模型菜单选项和当前选择完全一致。

#### AC-38：Codex/Claude 配置零改写

通过标准：除用户在既有 Manager 流程中明确执行的操作外，本集成不写 `config.toml`、`auth.json`、Claude 配置或官方安装文件。

验证方式与证据：端到端前后文件摘要、mtime 和配置语义 diff；没有变化才通过。

### 11. 性能、稳定性与回滚

#### AC-39：页面壳即时反馈且不冻结 Codex

通过标准：点击入口后 200ms 内出现本地壳/骨架；慢请求可取消并转为 stale/错误态；加载 100 项上限列表、切换模块和隐藏页面时 Codex 输入及原生导航保持可交互。

验证方式与证据：Playwright timing、请求取消、长列表和主线程响应记录。

#### AC-40：错误边界只卸载工作区

通过标准：工作区组件抛出未捕获错误时显示本地错误边界或卸载工作区；Codex renderer 不崩溃，原生导航和输入可恢复。

验证方式与证据：错误注入测试、renderer 存活状态和恢复截图。

#### AC-41：关闭与回滚不删数据

通过标准：用户关闭持久化工作区开关后入口消失、页面壳卸载、轮询和新派发停止；关闭状态跨 Manager/Codex 切换、重启和升级读取保持一致。既有 Multica Issue、映射、Skills 绑定和 Codex thread 保留且可从原生 Codex 访问。重新启用后从持久游标恢复，不重复执行，也不触发 legacy 下载/登录/daemon/runtime 生命周期。

验证方式与证据：关闭/重启/启用回放、数据计数、thread ID 和执行调用计数。

#### AC-42：许可和归属门禁

通过标准：

- 实现未直接复制 Multica UI 代码，或已提供适用授权和保留品牌的证据。
- 包含/分发 Multica 或衍生代码时，发行物携带完整上游 LICENSE、NOTICE、版权和归属。
- 用户文档注明 Built on Multica 和上游仓库地址。
- 使用锁定上游版本、可信资产 URL 和已验证摘要。

验证方式与证据：source/lock 清单、发行包内容检查、许可证审查记录和文档链接。任一必需许可条件不满足时禁止发布，即使功能测试通过。

## 分阶段最窄验证

以下命令名可按最终测试文件命名调整，但交付证据必须给出实际等价命令、退出码和通过数量。

### 阶段 0：契约与存储

```powershell
cargo test -p claude-codex-pro-core multica_workspace -- --nocapture
cargo test -p claude-codex-pro-core codex_execution -- --nocapture
```

只覆盖 typed DTO、状态机、幂等、迁移、许可/版本清单和两个 fake adapter。

### 阶段 1：入口、页面壳与只读 bridge

```powershell
cargo test -p claude-codex-pro-core --test cdp_bridge codex_multica -- --nocapture
cargo test -p claude-codex-pro-core --test bridge_routes multica_workspace -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
```

另运行专用 Playwright/CDP 用例验证入口位置、重注入、十模块路由、无 iframe、无外部页面依赖和样式隔离。此阶段不做 release 全构建。

### 阶段 2：CRUD 与看板

```powershell
cargo test -p claude-codex-pro-core multica_issue -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
```

仅验证 Issue/项目 CRUD、revision、拖拽、权限和 optimistic rollback。

### 阶段 3：Codex 原生执行

```powershell
cargo test -p claude-codex-pro-core codex_execution -- --nocapture
cargo test -p claude-codex-pro-core multica_execution_binding -- --nocapture
cargo test -p claude-codex-pro-core --test bridge_routes multica_execution -- --nocapture
```

再运行一次真实 Codex 单任务和一次小队/subagent 定向验收；不提前跑 workspace 全量回归。

### 阶段 4：自动化及恢复

```powershell
cargo test -p claude-codex-pro-core multica_autopilot -- --nocapture
cargo test -p claude-codex-pro-core multica_reconcile -- --nocapture
```

只验证自动化、租约、事件游标、部分失败、统计和 Skill 信任边界。

## 最终回归验收

所有分阶段验收通过后，最后一轮一次性运行：

```powershell
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo fmt --check
cargo test --workspace
cargo build --release
```

随后必须：

1. 结束可执行路径明确位于本项目构建目录且会阻塞新构建的旧 CCP 进程。
2. 确认默认 `target/release` 下最新 CCP 可执行文件存在并记录时间/摘要。
3. 使用该构建启动 CCP 和 Codex，重新注入最新脚本。
4. 完成 AC-03 至 AC-42 的真实 UI、原生 thread、配置隔离和进程验收。
5. 至少保存：入口/插件位置、十模块、看板、单 thread 映射、打开/继续、attempt lineage、小队父子关系、断线恢复和隔离回归截图/日志。

## 必需证据汇总

交付报告至少包含：

- 实际修改文件和数据迁移清单。
- 每阶段定向命令、退出码、通过/失败数量和修复记录。
- 最终五条全量命令的完整结果摘要。
- 默认 release 可执行文件路径、构建时间和摘要。
- CDP/Playwright 的 DOM 顺序、节点数量、几何、网络资源和十模块截图。
- Multica run/Issue 与 Codex thread/task/subagent 的脱敏映射及事件时间线。
- 幂等重复、409、三类部分失败、断线/游标 gap 和 orphaned 的恢复证据。
- 端到端进程树与关键配置摘要，证明没有 Codex app-server/runtime 注册或第二执行器、无供应商/代理/模型/Claude-Codex 配置改写；执行事件来自当前 Codex 页面 host。
- LICENSE、NOTICE、上游版本、资产哈希和用户归属文档检查结果。
- 每个 AC 编号的通过、未通过或不适用状态；不适用必须说明平台或能力原因。

## 失败条件

出现以下任一情况即不通过：

- 入口不在原生插件下方、项目上方，或重注入后出现重复入口/重复请求。
- 任一必需模块只是外链、iframe、静态占位或假数据。
- 工作区依赖 Multica Web/CDN/外部浏览器才能完成核心流程。
- 分配或执行没有创建 Codex 原生 task/thread，或使用了非页面原生入口、任何 Codex app-server/CLI/非 Codex Desktop 二进制、npm/Claude/其他 Provider/第二模型执行器。
- 重复请求创建多个 run、thread 或 subagent。
- “已派发”“HTTP 2xx”或缓存状态被显示为 completed/done。
- 打开/继续错误地创建新 thread，终态重跑覆盖旧 attempt。
- 409、断线、部分提交或事件 gap 导致状态丢失、重复执行或不可审计。
- renderer 可透传任意 URL/header/命令/路径，跨 workspace 读写，或不可信内容触发执行。
- token、API Key、Authorization、完整 prompt 或会话正文进入 DOM、日志、URL、截图或测试产物。
- 工作区改写供应商名称/Profile/Base URL、代理地址、模型选择或 Codex/Claude 配置。
- 工作区异常导致 Codex renderer、原生导航、输入框或 CCP 代理不可用。
- 工作区开关未持久化、关闭后仍残留入口/页面壳/轮询/新派发，或开启/关闭触发 CLI 下载、profile 创建、登录、daemon、Runtime 注册或 app-server 启动。
- Manager 主导航仍显示独立 `Multica Runtime` 页面，或移除该页面时连带删除了普通设置中的工作区启用/停用开关。
- 未使用最新默认 release 产物做最终验收，或只有静态源码判断而无运行证据。
- 上游许可、NOTICE、品牌/归属或商业分发门禁不满足仍发布集成产物。

## 非范围检查

以下内容不是本阶段完成条件，但任何实现不得阻碍后续扩展：

- Multica 收件箱、聊天、附件预览和移动端。
- 将 Multica server/database 合并到 CCP 或 Codex 进程。
- 用非原生多进程方式模拟 Codex subagent。
- 自动安装未知 Skills、hooks 或 MCP。
- 修改 Codex 官方模型菜单、供应商路由或 Claude 官方文件。

## 已知约束

- Codex 页面原生 task/thread/subagent API 可能随版本变化，必须以实时 capability detection 和契约测试为准；不可用时允许明确降级，但不允许注册 app-server、改走 CLI 或启动第二执行器。
- Multica API schema 和许可条款可能随锁定版本变化；升级前必须重新生成兼容性与许可证据。
- macOS 的窗口结构、注入和进程监管与 Windows 不完全相同；启用该平台发布时必须补齐等价运行证据。
- 大规模 workspace 的统计和看板性能需要有界分页；本阶段不以一次加载全部历史作为验收方式。
