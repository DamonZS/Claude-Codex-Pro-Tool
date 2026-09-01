# Codex 内嵌 Multica 工作区集成

## 背景

CCP 需要把本地 Multica 控制面和工作区直接接入 Codex App：用户在 Codex 左侧导航进入工作区，完成任务创建、分配、看板流转、执行、打开和继续，而不再把外部 Multica 桌面端或浏览器作为主入口。本集成运行的是随 CCP 提供的本地控制面/编排逻辑，不要求安装或运行 Multica 的完整 Web、Desktop、server 或独立执行服务。

Multica 与 Codex 的职责必须保持清晰：本地 Multica 只负责工作区、Issue、项目、成员、智能体、小队、自动化、状态持久化、审计和调度；真正的 AI 执行必须由当前 Codex 页面已经提供的原生 task/thread/subagent 和 Skills 能力承担。CCP 只做两者之间的本地适配、映射和对账。不得注册第二个 Codex Runtime，不得由 Multica daemon 或其他后台进程启动、托管或连接 `codex.exe app-server`；也不得启动 npm 旧 CLI、Claude CLI、其他模型 Provider、第二套模型客户端或隐藏执行器。

本规格建立在以下文档之上：

- `spec/multica-runtime-adapter.md`：仅定义可选外部连接的配置隔离边界；它不是本地工作区的运行前置条件。
- `spec/multica-managed-runtime.md`：仅定义显式开启后的 legacy 高级兼容能力；其下载、登录和 daemon 监管要求不适用于本地工作区默认启动路径。
- `spec/codex-injection-content-boundary.md`：禁止注入层改写 Codex 原生内容。
- `spec/remove-codex-model-selection-injection.md`：禁止 CCP 注入或改写 Codex 模型选择。

本规格是后续增量。它只扩展“在 Codex 内提供本地 Multica 工作区并通过当前 Codex 页面原生能力执行任务”的范围，不弱化上述文档中的供应商、代理、凭据、进程和内容隔离约束。

### 本地执行边界（不可变）

- 本地 Multica 是控制面和编排层，不是模型运行时。它可以保存任务、分配、租约、映射和审计，但不能执行模型请求。
- 本集成不调用 `register_managed_codex_runtime` 或等价注册 API，不建立独立的 `CodexAppServerTransport`/JSON-RPC 通道，也不把 `codex.exe app-server` 作为依赖、子进程或托管服务。
- 执行入口只能是用户当前打开的 Codex 页面及其原生 host API（task/thread/subagent/Skills）。CCP 的 typed adapter 只能转发到该页面已暴露且经能力探测确认的原生入口。
- 当前页面原生能力不可用时，操作必须返回 `unsupported` 或排队等待用户回到可用页面；不得改走 CLI、shell、HTTP 代理、另一个窗口或伪造执行结果。
- 文档中的“运行时”仅表示 Multica 控制面和当前 Codex 页面能力的状态视图，不表示需要注册或启动新的进程。

## 目标

### 本次包含

- 在 Codex App 左侧原生“插件”入口下方、原生“项目”入口上方插入一个单实例的 `我的任务` 入口，点击后默认进入 `my-issues`。
- 在 Codex 主内容区直接展示 CCP 自有、本地打包、非 iframe 的全宽任务看板；不再叠加 `Multica 工作区 / Local Multica Workspace` 顶部壳、永久模块侧栏或“关闭”按钮。
- 首批完整提供十个模块：`我的任务`、`任务`、`项目`、`自动化`、`智能体`、`小队`、`统计`、`运行时`、`Skills`、`设置`。
- 十个模块通过看板工具栏中的紧凑模块菜单和上下文动作进入，保留稳定路由和完整能力，但不以第二套永久导航挤占主内容宽度。
- 支持 Issue 的创建、读取、编辑、归档/取消、筛选、分配、项目归属、父子关系和看板状态流转。
- 支持把 Issue 分配给智能体、小队或成员，并将可执行分配映射为 Codex 原生 task/thread/subagent。
- 支持从 Multica 任务打开已有 Codex 对话、继续执行、查看执行状态和创建新的重试 attempt。
- 支持 Multica 项目、自动化、智能体、小队、统计、运行时和 Skills 的核心工作流；Skills 是一等执行能力，不是只读目录或装饰性标签。
- 使用稳定映射、revision/CAS、幂等键、事件游标和对账机制保证 Multica 状态与 Codex 执行状态可恢复、可审计。
- 默认启用“我的任务”入口并初始化本地 Multica 控制面；新版 Codex 页面 Host 不可用时仍加载本地任务数据和看板，只把必须依赖原生 thread/subagent/Skills 的动作标为不可用或排队。
- 在设置中保留可持久化的“启用 Multica 工作区”开关；新安装默认开启，升级保留用户上次选择。
- Manager 只保留本地工作区开关、诊断和明确标记的 legacy 高级兼容入口，不在主导航保留独立 `Multica Runtime` 页面，也不复制日常任务工作区；本地工作区不依赖 Manager、外部 server 或独立 daemon 才能执行。

### 成功结果

用户点击 Codex 左侧“我的任务”后直接看到全宽七列真实看板，无需理解或关闭额外的 Multica 页面壳。用户可以建立一个 Issue，将其分配给当前 Codex 页面提供的原生 task/thread/subagent 执行者，选择本次执行需要的受信任 Codex Skills，并看到 CCP 只创建一个原生执行对象、把最终 Skill 清单交给当前页面的 Codex host。在看板中可以观察状态变化、核对页面原生能力实际加载的 Skills，并通过真实激活 Codex 原生对话行回到同一 thread；切换到任意原生项目或对话时只隐藏看板，后台任务、事件同步和对账继续运行。整个流程不会注册新的 runtime，不会改写 Codex 模型、CCP 供应商、代理地址或 Codex/Claude 配置。

## 非目标

- 不把 Multica Web、Desktop 或第三方页面通过 iframe、WebView 套娃或外部浏览器嵌入 Codex。
- 不把跳转 Multica 网页作为任何核心工作流的完成方式。
- 不复制或下载另一份 Codex CLI，不使用 PATH 中的 npm `codex.cmd`，不运行 Claude CLI、其他模型调用器或非 Codex Provider；更不得注册、启动、托管或连接任何新的 Codex Runtime 或 `codex.exe app-server`。
- 不让 Multica 直接持有 CCP 的供应商 API Key、代理地址、Codex 登录材料或 Claude 登录材料。
- 不修改 Codex 原生模型菜单、模型状态、请求模型字段、供应商/Profile、Base URL、`config.toml`、`auth.json` 或 Claude 配置。
- 不翻译、替换、隐藏或重排 Codex 原生会话、项目、输入框、插件内容和标题栏控件。
- 不把完整 Multica Web/Desktop/server、数据库或 daemon 合并到 Codex renderer、Tauri 主进程或 CCP 代理进程；本地控制面只通过受控适配层读写自身状态。
- 不在 CCP/Codex 默认启动路径自动下载或安装 Multica CLI，不自动创建托管 profile，不自动登录或启动/监管 Multica daemon；legacy 高级兼容能力必须由用户单独显式开启。
- 不把独立 `Multica Runtime` 托管页面保留在 Manager 主导航；本地工作区的启用/停用设置不得依赖该页面存在。
- 不在本阶段实现上游可选的收件箱、聊天、附件预览或移动端；后续可按相同边界增量接入。
- 不在用户无感知的情况下安装第三方 Skill、执行 hook、运行脚本或扩大工作区权限。
- 不因本集成删除既有 Multica 数据、Codex 对话或执行审计。

## 领域边界与权威状态

### Multica 权威范围

本地 Multica 控制面是以下编排数据的权威来源（如配置了同步端点，同步只作为受控数据源，不改变执行边界）：

- workspace、成员和权限；
- Issue、项目、父子关系、优先级、日期和自定义状态；
- 智能体、小队、分配关系和自动化定义；
- 调度请求、run/attempt 记录、审计事件和业务状态；
- 统计口径及历史协作数据。

### Codex 权威范围

Codex App 是以下数据的权威来源：

- 原生 task/thread/subagent 是否真实创建；
- thread 当前可否打开、继续或接收后续消息；
- 原生执行的运行、完成、失败、取消和用户交互状态；
- 当前 Codex 支持的模型、能力、项目上下文和会话内容。
- 当前可用的 Codex Skills、Skill bundle 能力、加载结果和受信任执行边界。

### CCP 适配层职责

CCP 只负责：

- 对本地 Multica 控制面和 Codex 页面原生能力进行显式、可测试的适配；
- 持久化两侧稳定 ID、attempt lineage、revision、事件游标和幂等状态；
- 将 Codex 真实事件投影回 Multica，而不是自行推测成功；
- 在冲突、断线和部分失败时对账并显示可恢复状态；
- 向注入页面提供经过鉴权、脱敏和限流的窄 bridge 接口。

任何一侧暂时不可达时，另一侧的已确认状态仍可读取，但不得把缓存或中间状态伪装为实时成功。

## 用户视角工作流

### 1. 首次进入

1. CCP 启动并注入 Codex 后，在原生“插件”按钮下方、原生“项目”按钮上方显示 `我的任务`。
2. 工作区开关在新安装时默认为启用，并以独立布尔设置持久化；升级和重启必须保留用户已保存的值。启用时仅初始化本地控制面，不下载 Multica CLI，不登录或启动 daemon，不注册或启动新的 Codex runtime，也不另起完整 Multica 服务。
3. 点击入口后默认直接打开当前 workspace 的 `my-issues` 七列看板；其他模块通过看板工具栏中的紧凑模块菜单或上下文动作进入，不显示永久模块侧栏。
4. 本地数据或可选同步端点不可用时显示脱敏诊断和重试；不要求用户先安装、登录或启动独立 daemon。
5. 当前 Codex 页面 Host 能力缺失、改版或探测失败时，本地任务看板仍须加载并允许不依赖 Host 的查询、筛选和业务状态操作；仅执行、继续、Skills 实际加载等 Host 动作显示明确降级。任何故障不得遮挡 Codex 原生导航或阻止用户返回普通 Codex 对话。
6. 本地 bridge 未安装、Launcher 未启动、binding 缺失、请求超时或本地传输失败时，点击“我的任务”不得隐藏或冻结原生主内容，也不得把失败渲染成“无任务”。入口必须显示可恢复的不可用状态并提供直接重试；只有 bootstrap 与当前页面的首个 `my-issues` 查询均已获得有效本地响应后，才允许接管主内容显示看板。

### 2. 创建和分配任务

1. 用户在“任务”或项目详情中创建 Issue，至少输入标题；描述、优先级、项目、父任务、日期、位置、自定义状态均为可选项。
2. 未分配的 Issue 只保存在 Multica，不自动创建 Codex thread。
3. 用户分配已启用的智能体/小队，或显式点击“执行”时，CCP 先以幂等键在 Multica 预留 run/attempt，再创建 Codex 原生执行对象。
4. 创建成功后，CCP 原子保存映射并回写 Multica；创建失败则将预留 attempt 标记为明确失败或可重试，不留下“运行中”的假状态。
5. 单纯拖动看板状态不隐式启动模型执行；只有分配触发策略或显式执行动作可以派发。

### 3. 打开和继续

1. 有绑定 thread 的 Issue 显示“打开对话”和“继续执行”。
2. “打开对话”必须按稳定 `codex_thread_id` 定位所属项目下的 Codex 原生对话行，必要时先展开对应项目，再触发该原生行自身的激活行为，并以原生 active row/当前 thread 状态确认成功；不能只隐藏看板、只改 URL/history、直接改 React store 或伪造已打开结果。
3. “继续执行”向同一个非终态 thread 发送经过用户确认的后续任务上下文，并记录 Multica 审计事件。
4. thread 已终态、不可继续或用户选择重跑时，新建 `attempt_no + 1`，保留 `parent_thread_id` 和旧 attempt，不覆盖历史映射。
5. 原生对话行真实激活后立即隐藏工作区视图并显示该对话，后台执行、事件同步和对账保持运行；找不到或无法激活已绑定 row 时保留看板并显示明确错误，标记 `orphaned` 或提供对账，禁止静默新建。

### 4. 看板流转

- `my-issues` 首屏使用全宽真实看板，不渲染列表占位或静态示例。顶部只保留看板自身标题与工具栏，不显示 `Multica 工作区 / Local Multica Workspace` 顶栏、全局状态条、“刷新/关闭”外壳或永久模块侧栏。
- 看板工具栏至少包含 `全部`、`已分配`、`我创建的`、`我的智能体和小队`，默认选中 `已分配`；右侧显示真实的工作中智能体数量、筛选、显示方式、看板模式和紧凑模块菜单。
- Issue 状态类别至少支持：`backlog`、`todo`、`in_progress`、`in_review`、`done`、`blocked`、`cancelled`。
- 七列视觉顺序固定为 `待规划`、`待办`、`进行中`、`审核中`、`已完成`、`已阻塞`、`已取消`；每列显示状态图标、真实数量、更多/新增动作和 `无任务` 空态。
- 卡片展示真实编号、标题、摘要、负责人或执行者、更新时间和独立执行状态；hover/focus 预览不得改变列尺寸或推动看板重排。
- 列使用稳定宽度和全高布局；可用宽度不足以容纳七列时在看板内容区横向滚动，不压缩成不可读窄列，也不让页面本身出现第二条横向滚动。
- 拖拽使用 `expected_revision` 更新；服务端 revision 已变化时拒绝覆盖，刷新卡片并提示冲突。
- `done` 不能仅由“请求已派发”推导；自动进入 `done` 必须来自 Codex 已确认完成事件和 Multica 成功提交。
- `blocked`、`cancelled` 和人工 `in_review` 保留操作者、时间和原因。
- 用户可以手动调整业务状态，但界面必须将“业务状态”和“执行状态”分开展示。

## 信息架构与功能矩阵

工作区内部使用稳定路由键，不接管 Codex 自身 URL 路由。`我的任务` 是唯一常驻 Codex 入口并默认进入 `my-issues`；其余模块通过看板工具栏中的单个紧凑模块菜单或业务上下文动作进入，禁止再渲染永久竖向模块栏。首批模块在紧凑菜单中的顺序固定如下：

| 顺序 | 模块 | 路由键 | 最低可用能力 |
| --- | --- | --- | --- |
| 1 | 我的任务 | `my-issues` | 当前身份相关任务、筛选、打开、继续、个人负载与待处理状态 |
| 2 | 任务 | `issues` | 列表/看板、创建、编辑、分配、拖拽、父子任务、执行和 attempt 历史 |
| 3 | 项目 | `projects` | 项目创建/编辑、状态、日期、成员、任务归组、进度和项目详情 |
| 4 | 自动化 | `autopilots` | schedule/webhook/api 触发定义、启停、手动运行、run 历史和失败诊断 |
| 5 | 智能体 | `agents` | 智能体定义、启停、能力/Skills、并发策略和 Codex 原生执行映射状态 |
| 6 | 小队 | `squads` | 负责人、成员、分工、父子执行拓扑、队列和汇总状态 |
| 7 | 统计 | `usage` | Issue、attempt、成功率、耗时、队列与智能体/小队维度的审计统计 |
| 8 | 运行时 | `runtimes` | 本地 Multica 控制面与当前 Codex 页面原生能力的只读状态、版本和故障信息；不管理或注册进程 |
| 9 | Skills | `skills` | 可用 Codex Skill、来源、信任/安装状态、任务/智能体绑定、页面原生能力和执行加载结果；不隐式安装 |
| 10 | 设置 | `settings` | 持久化工作区启用/停用开关、workspace 默认值、分配/派发策略、通知、并发与同步设置；高级运维打开 Manager |

模块必须有加载态、空态、错误态、过期态和无权限态。模块不可用时显示具体原因，不能渲染虚假示例数据或只有外链的占位页。

## Codex 导航与页面壳

### 左侧入口

- 注入端优先复用已有 `pluginEntryButton()` 和 `selectors.pluginNavButton` 定位原生插件入口，文案匹配 `插件|Plugins` 仅作降级锚点。
- 使用插件按钮父节点的 `insertBefore(multicaEntry, pluginEntry.nextSibling)`，确保 DOM 顺序和视觉顺序均为“插件 → 我的任务 → 项目”（在存在项目入口的原生导航中）。
- 入口使用稳定标识，例如 `data-ccp-multica-nav` 和注入版本号；重复扫描、窗口切换、React 重绘和重新注入后全页最多存在一个有效入口。
- 插件锚点暂不可用时执行有界重试；达到上限后记录诊断并等待下一次明确导航变化，不得插入到不相关区域或无限轮询。
- 点击任意 Codex 原生导航项、原生项目行、项目内对话行或“新对话”时，只隐藏工作区视图并恢复原生内容；必须让原生点击事件继续完成，不能 `preventDefault`、`stopPropagation`、修改原生节点文本/路由或调用完整 cleanup。视图隐藏不得停止已运行/排队任务、本地控制面、后台事件同步、租约、对账或自动化。
- 入口具备可见 focus、键盘激活、选中状态和可读无障碍名称，尺寸与 Codex 原生导航一致。

### 工作区页面壳

- 页面壳由 CCP 本地资源渲染，可采用独立 React bundle，但 host 挂载、卸载和导航协调仍由 `renderer-inject.js` 负责。
- 所有脚本、样式和图标随 CCP 构建打包，不从 CDN 或 Multica Web 动态加载前端代码。
- 页面壳挂载在 CCP 自有根节点，可使用 Shadow DOM 或等效样式隔离；样式不得污染 Codex 原生输入框、菜单、标题栏、插件页和会话内容。
- 页面壳占用 Codex 原生侧栏之外的完整主内容区域，保留原生左侧导航和 Windows/macOS 窗口控件，不使用模态框承载日常工作流。
- 页面壳不得渲染额外的产品顶栏、workspace 名称/status 横条、全局刷新/关闭按钮或永久模块侧栏；`my-issues` 直接从“我的任务”看板标题、筛选工具栏和七列内容开始。
- 十模块切换使用一个可键盘操作、有 tooltip 和选中反馈的紧凑图标菜单；关闭菜单不会关闭工作区，选择模块只切换 CCP 自有路由。
- 不创建 iframe，不嵌入外部 WebView，不把完整 Multica HTML 注入为不受控字符串。
- 窗口缩放、侧栏折叠和全屏变化时重新测量可用区域；固定工具栏、看板列和表单使用稳定响应式约束，不发生文字或控件重叠。
- 工作区内部路由存入 CCP 自有状态；不得改写 Codex URL、history、原生 React store 或项目选择状态。
- 原生导航造成的视图隐藏只切换可见性并恢复原生 main，不卸载后台编排或移除持久事件/对账状态；只有用户将持久化“我的任务”增强开关设为关闭时才执行完整 cleanup，移除入口、页面 host、UI 监听器和临时 DOM。完整 cleanup 也不得删除或重建 Codex 原生内容及已持久化任务数据。

## 数据模型

### 工作区连接

```text
CodexMulticaWorkspaceBinding
- binding_id
- multica_connection_id
- workspace_id
- workspace_slug
- enabled
- default_route
- last_event_cursor
- last_successful_sync_at
- revision
- created_at
- updated_at
```

### Issue 与执行映射

```text
CodexMulticaExecutionBinding
- binding_id
- workspace_id
- issue_id
- multica_task_id
- multica_run_id
- codex_thread_id
- codex_task_id
- parent_thread_id
- execution_kind        # thread | subagent
- executor_kind         # agent | squad | member
- executor_id
- attempt_no
- idempotency_key
- state
- multica_revision
- codex_revision
- last_event_id
- last_error_code
- created_at
- updated_at
- completed_at
```

### Skill 绑定与执行快照

```text
CodexMulticaSkillBinding
- binding_id
- workspace_id
- scope_kind             # task | agent
- scope_id
- skill_ref
- source_kind
- trust_state
- enabled
- revision
- created_at
- updated_at

CodexMulticaAttemptSkillSnapshot
- snapshot_id
- execution_binding_id
- attempt_no
- requested_skill_refs
- resolved_skill_refs
- resolved_manifest_digest
- codex_page_loaded_skill_refs
- resolution_status
- resolution_error_code
- created_at
- loaded_at
```

Skill 绑定是用户配置，attempt Skill 快照是不可变执行证据。派发时按“任务级显式选择 > 智能体默认绑定”计算最终清单；同一 Skill 重复引用必须按稳定 ID 去重，来源或摘要冲突必须阻止派发。当前 Codex 页面原生 API 返回的实际加载结果不得覆盖请求或解析清单，三者必须分别保留以便审计和故障恢复。

约束：

- `workspace_id + issue_id + attempt_no` 唯一。
- `idempotency_key` 唯一，重复请求返回同一已知结果，不再创建 thread。
- 一个 Issue 可以有多个 attempt，但同一时刻最多有一个非终态主 attempt，除非用户明确选择并行执行。
- Squad 的负责人 thread 与成员 subagent 分别持有映射，并通过 `parent_thread_id`、`parent_attempt_id` 组成 lineage。
- 映射保存在 CCP 独立存储中；若 Multica 支持外部引用元数据，可同步只含稳定 ID 的镜像，但不得依赖单侧镜像完成恢复。
- 凭据、完整 prompt、完整会话正文和 API Key 不进入映射表或普通诊断日志。

## 执行状态机与映射

Multica `AgentTask` 执行状态至少支持：

```text
queued
dispatched
waiting_local_directory
running
completed
failed
cancelled
```

附加的 CCP 对账状态可以包含 `binding_pending`、`stale`、`orphaned` 和 `reconciling`，但不得冒充 Multica 原生终态。

状态规则：

1. Multica run 预留成功后进入 `queued` 或 `binding_pending`。
2. Codex 原生 create 返回稳定 ID 且映射提交成功后，才可进入 `dispatched`。
3. Codex 要求选择/恢复项目目录时进入 `waiting_local_directory`，不得自动猜测目录。
4. 收到 Codex 权威运行事件后进入 `running`。
5. 只有 Codex 权威完成事件和 Multica CAS 写入均成功，投影视图才显示 `completed`；回写暂时失败时显示“已完成，待同步”。
6. Codex 创建失败、执行失败、被取消或 thread 丢失时分别记录稳定错误码和可重试性。
7. 用户取消时先记录取消意图，再调用 Codex 原生取消能力；无法确认取消时显示 `cancel_pending`，不得直接标记 `cancelled`。
8. 每次状态变化包含 `event_id`、`correlation_id`、来源、旧/新状态、revision 和时间，重复事件必须可去重。

## Codex 页面原生执行适配器

Core 层新增窄接口 `CodexExecutionService`（名称可按现有约定调整），只封装当前 Codex 页面/renderer host 实际暴露的原生能力。该接口是适配层，不是新的 runtime 注册表或进程传输层；生产实现必须绑定当前已打开的 Codex 页面上下文，并通过其原生 task/thread/subagent/Skills API 执行：

```text
capabilities()
list_skills()
resolve_skills(skill_refs)
create_thread(request, idempotency_key)
create_subagent(parent_thread_id, request, idempotency_key)
open_thread(thread_id)
continue_thread(thread_id, request, idempotency_key)
cancel_execution(thread_id, execution_id)
execution_status(thread_id, execution_id)
subscribe_events(cursor)
```

实现约束：

- 必须基于当前页面已验证的 Codex 原生 task/thread/subagent、`agent-skill-v1`、`skill-bundles-v1` 接口或事件，不以 DOM 文案点击作为唯一执行通道。
- 禁止调用 `register_managed_codex_runtime` 或任何等价注册 API；禁止建立独立 `CodexAppServerTransport`、JSON-RPC app-server 通道，禁止由 Multica daemon/worker 启动或托管 `codex.exe app-server`。
- CCP 不直接通过 shell、隐藏终端、CLI、HTTP 代理或第二个窗口派发任务；当前页面 host 不可用时返回 `unsupported`，不得降级到第二执行器，也不得伪造成功事件。
- 页面 Host 的能力探测结果只约束原生执行动作，不是本地工作区 bootstrap 或 `my-issues` 查询的前置条件。Host 缺失、改版或暂时失败时仍返回本地 workspace/Issue 数据并渲染七列看板；依赖 Host 的按钮显示不可用、排队或重试状态。
- 创建请求只包含用户确认的 Issue 上下文、workspace/project 引用、已安装且受信任的 Skill 引用和执行策略，不携带 Multica/CCP 凭据。
- 派发前必须解析每个 Skill 引用并生成稳定清单；当前页面原生 API 返回的实际加载 Skill 集必须写入 attempt 审计。缺失、未受信任、重复冲突或页面能力不支持的 Skill 会阻止派发，不得静默忽略或自动安装。
- `open_thread` 必须定位并真实激活匹配 `codex_thread_id` 的 Codex 原生侧栏 row，以原生 active row 和当前 thread 状态作为成功条件；不得用修改 URL/history/React store 或仅隐藏工作区来代替。激活成功后只隐藏工作区视图，后台任务与对账继续；`continue_thread` 必须复用同一 ID，除非状态机明确创建新 attempt。
- 任何原生请求返回“已接受”只表示派发成功，不表示任务完成。

## 智能体与小队执行

- Multica 智能体描述协调策略、角色、允许的 Skills、并发上限和默认项目，不配置第二套模型 Provider。
- 单智能体任务映射为一个 Codex 原生 thread/task。
- 小队任务先创建负责人 thread；成员工作通过该 thread 的 Codex 原生 subagent 能力创建，并记录父子映射。
- subagent 创建、消息、取消和完成均由 Codex 原生能力产生；Multica 只保存计划、指派和结果摘要。
- 任一成员失败不自动覆盖其他成员状态；负责人汇总后再推进 Issue 到 `in_review` 或 `done`。
- 重复收到小队派发事件时必须按成员级幂等键去重，不能重复创建 subagent。
- Codex 不支持原生 subagent 时，小队执行显示不支持；不得用多个 CLI 进程模拟。

## Codex Skills 执行

- Skills 页面展示当前 Codex 页面原生能力实际发现的 Skills，至少包含稳定 ID、名称、来源、版本/摘要、安装位置类别、信任状态、兼容能力和最近加载结果；不得把 Multica 自定义标签伪装成已安装 Skill。
- 用户可以把一个或多个受信任 Skill 绑定到单次任务或智能体。任务级选择覆盖默认绑定时必须在派发确认中明确展示最终 Skill 清单。
- `CodexExecutionService::resolve_skills` 只接受稳定 Skill 引用，交叉验证当前 Codex 页面 inventory 与 CCP 现有 Skill 信任/审查状态，再通过 `agent-skill-v1` 或 `skill-bundles-v1` 交给同一页面 host。
- 每个 attempt 保存请求的 Skill 引用、解析后的不可变摘要和页面原生 API 返回的实际加载结果，但不保存完整 Skill 正文、凭据或任意命令。
- 未安装或未受信任 Skill 只能打开现有 CCP Skill 审查/安装流程；任务派发不能隐式下载、写文件、执行 hook 或扩大 MCP 权限。

## 自动化

- 自动化支持 `schedule`、`webhook` 和 `api` 触发、启停、手动运行、运行历史和 agent/squad 分配。
- 触发发生在 Multica；CCP 在线且持有有效租约时，将待执行 run 交给 Codex 原生适配器。
- CCP 离线或当前 Codex 页面不可用时，run 保持排队/等待用户回到可用页面，不得由 Multica 另起模型执行器。
- 领取使用租约和幂等键；租约过期后的重新领取必须先查询已有映射，确认没有 Codex 执行对象后才能创建。
- 若启用远端同步，webhook/API 入口的鉴权、签名和限流由同步端点负责；本地模式不暴露公网监听端口，Codex 注入页面也不暴露公网监听端口。
- 手动运行必须显示即将使用的 workspace、Issue 模板和执行者，用户确认后再派发。

## Bridge 接口

注入页面只能通过现有 CDP `Runtime.addBinding` 通道调用 CCP Core。页面不直接请求远端 Multica 服务，不持有 Multica token，也不依赖 CORS；本地模式直接访问受控的本地控制面。

建议新增以下受控路径：

```text
/multica/workspace/bootstrap
/multica/workspace/query
/multica/workspace/mutate
/multica/execution/command
/multica/events/poll
/multica/reconcile
```

### 查询请求

```json
{
  "workspace_id": "stable-id",
  "resource": "issues|projects|autopilots|agents|squads|usage|runtimes|skills|settings",
  "operation": "allowlisted-query",
  "cursor": "opaque-cursor",
  "limit": 50,
  "filters": {}
}
```

### 变更请求

```json
{
  "command_id": "uuid",
  "workspace_id": "stable-id",
  "entity": "issue|project|autopilot|agent|squad|settings",
  "action": "allowlisted-command",
  "entity_id": "stable-id",
  "expected_revision": 12,
  "payload": {}
}
```

### 执行请求

```json
{
  "command_id": "uuid",
  "workspace_id": "stable-id",
  "issue_id": "stable-id",
  "action": "start|open|continue|cancel|retry|reconcile",
  "attempt_id": "optional-stable-id",
  "expected_revision": 12
}
```

接口约束：

- `resource`、`operation`、`entity` 和 `action` 均由 Core 枚举白名单解析，不接受任意 URL、HTTP method、header、shell、文件路径或上游请求体透传。
- Core 根据已保存的 `connection_id/workspace_id` 校验访问范围，不信任 renderer 传入的租户、角色或权限结论。
- 请求体、响应体、列表数量和轮询频率有固定上限；超限返回稳定错误码。
- 所有 mutation/execution 命令要求 `command_id`；并发更新要求 `expected_revision`。
- 响应统一包含 `status`、`data`、`revision`、`cursor`、`stale`、`error_code`、脱敏 `message` 和 `correlation_id`。
- bridge 日志只记录路径、操作类别、稳定 ID、耗时和结果状态，不记录 token、Authorization、Cookie、完整 prompt、完整响应正文或用户会话正文。
- HTTP helper fallback 不扩展为任意 Multica API 代理；工作区业务统一走鉴权后的 CDP bridge。

## 同步、幂等与恢复

- 每个 workspace 持有独立事件游标，事件按 `event_id + revision` 去重并按实体串行应用。
- 首次加载先读取服务快照，再从快照 revision 订阅增量；断线重连从最后确认游标继续。
- 游标过期或出现 gap 时触发有界全量对账，不无限重放历史。
- mutation 采用 optimistic UI 时必须保留原值；服务拒绝或超时后回滚并显示真实状态。
- 对“Multica 已预留、Codex 未创建”“Codex 已创建、映射未提交”“Codex 已完成、Multica 未回写”三类部分失败分别提供补偿流程。
- 对账先查询本地映射和 Codex 原生状态，再决定绑定、重试或标记 orphaned；不能只根据时间自动重建 thread。
- 应用退出、注入重载或 Codex 页面刷新不丢失未完成 command；恢复后同一 `command_id/idempotency_key` 只能得到一个执行对象。
- 用户从“我的任务”切换到原生项目、对话、新对话或插件只改变可见视图，不改变 run/attempt 状态，不取消 command、租约、事件同步、对账或自动化；再次打开“我的任务”时按当前游标恢复视图。
- 离线快照仅可读并标记 `stale`；离线时不盲目排队不可见写操作。用户重试必须复用原 command ID 或明确创建新命令。

## 默认启用与 Manager 边界

- 使用独立持久化布尔设置 `multica_workspace_enabled`（最终字段名可遵循现有设置命名，但语义不得改变）。新安装缺少该字段时默认 `true`；升级、重启、刷新和重新注入均保留用户已保存的 `true/false`，不得以“默认开启”为由回填覆盖。
- 持久化开关控制整项“我的任务”增强，包括 Codex 左侧入口、页面 host、本地事件轮询/对账和新任务派发。只有用户明确把该开关设为 `false` 才执行完整 cleanup：移除入口和页面 host，取消或暂停本地轮询与尚未派发的调度；不删除 workspace、Issue、映射、Skills 绑定或既有 Codex thread。
- 点击 Codex 原生导航或由工作区内部“打开对话”跳回原生 thread 属于临时视图隐藏，不改变持久化开关，也不得调用完整 cleanup、停止后台控制面、取消已排队/执行任务、停止事件同步/对账/自动化或释放任务租约。允许暂停纯展示用的前台请求，但后台权威状态必须继续推进。
- 开关不控制供应商、代理、Codex/Claude 启停、模型选择、完整 Multica server、CLI 下载、登录、daemon、`codex.exe app-server` 或任何 Runtime 注册。开启和关闭都不得调用这些生命周期或配置写入路径。
- Codex 启动不得同步等待 Multica 同步、登录、页面 Host 或远端服务；也不得因开关默认为 `true` 自动下载/安装 Multica CLI、创建托管 profile、登录、启动 daemon 或注册 Runtime。入口先可见，本地 `my-issues` 看板独立加载；页面 Host 故障只在依赖原生执行的控件附近显示降级，不得用全页错误或顶部状态壳替代本地任务看板。
- bridge 不可达与页面 Host 不可达是不同故障：前者表示本地控制面无法响应，必须 fail-open 回到原生 Codex，并在入口提供重试；后者不应阻断已可用本地控制面中的看板查询。两类故障均不得影响供应商、代理、模型选择、原生输入或普通导航。
- Manager 主导航不保留独立 `Multica Runtime` 页面。Manager 的普通设置区域必须保留“启用 Multica 工作区”开关和只读诊断；历史外部连接、下载安装、登录和 daemon 监管只可作为默认关闭、用户显式进入的 `legacy/高级兼容` 能力存在，且不是本地工作区的执行前置条件。
- Codex 工作区负责：日常 workspace、Issue、项目、自动化、智能体、小队、统计、能力只读视图、Skills 和普通设置。
- Codex 工作区设置与 Manager 必须读写同一个持久化开关来源；不得维护两份互相覆盖的值。其他高级配置动作可调用现有 `/manager/open` 打开 Manager。

## 安全与隔离

- Multica token、登录凭据和 daemon secrets 只存在于 Core/受保护存储，不进入 renderer DOM、localStorage、sessionStorage、URL 或截图。
- 所有写操作由 Core 校验 workspace 权限、实体归属、revision 和允许字段；不接受 renderer 自报管理员权限。
- Issue 描述、附件名称、错误信息和上游 Markdown 均视为不可信内容，使用结构化渲染和严格转义，禁止执行 HTML、脚本、命令或 MCP 指令。
- 外部 URL 只可在明确的用户动作和协议/主机白名单后交给系统打开；任何核心流程不得依赖外链。
- Skills 只展示和绑定受信任清单；安装或运行未知 Skill 必须走现有审查流程，不能随任务分配自动执行。
- 自动化、事件轮询和重试有连接级与全局并发上限，避免重复执行和阻塞 Codex 主界面。
- 本集成不得停止、重启、注入或监管 Codex 进程；工作区开关只暂停本地 Multica 页面壳、编排、轮询和新派发。legacy 同步/daemon 开关必须独立，且其停止范围只能限于归属已验证的 CCP 自有进程。
- 工作区开启、关闭、派发和对账不得调用供应商切换、代理启动/改写、模型增强或 Claude 配置命令。

## 第三方许可与来源约束

Multica 上游当前使用包含附加条件的 `Multica License`，并非可忽略附加条件的标准 Apache-2.0。实现和发布必须设置许可门禁：

- 默认采用“本地 Multica 控制面 API + CCP 原创工作区 UI”，不直接复制 `apps/web`、`apps/desktop`、`apps/mobile`、`packages/views` 或 `packages/ui` 的界面代码；可选同步端点不能成为 Codex 执行前置条件。
- 发行包若包含 Multica 或衍生代码，必须携带上游完整 LICENSE、NOTICE、版权和归属信息，并在用户文档注明 “Built on Multica” 及上游仓库地址。
- 若实现复用或派生 Multica UI，必须保留 Multica 名称、Logo 和归属；任何品牌豁免或商业分发许可必须在发布前取得并留存可审计证据。
- 如提供可选同步组件，下载安装只能使用锁定版本、可信官方来源和已验证摘要，不执行仓库中的任意安装脚本；不得下载或注册 Codex CLI/app-server。
- 许可审查未通过时可以开发仅 API 适配和本地验证，但不得把受限组合产物发布为正式构建。

## 性能与可用性

- 点击入口后 200ms 内显示本地页面壳或骨架，不等待网络后才反馈。
- 列表默认分页且单次不超过 100 项；看板按项目/过滤器有界加载，禁止无界拉取完整历史。
- 同一 workspace 的轮询和刷新去重；视图隐藏时可降低或停止纯 UI 展示刷新，但后台任务执行、事件同步、租约、对账和自动化保持运行，恢复视图时按持久游标补齐展示。
- 慢请求可取消，超时后保留最后成功快照并标记过期；不得冻结 Codex 输入和原生导航。
- 页面壳任何未捕获错误由错误边界隔离，失败时只卸载工作区 UI，不导致 Codex renderer 崩溃。
- 尊重 `prefers-reduced-motion`；状态、优先级和错误不能只用颜色表达。

## 技术落点

预计实现范围如下，实际文件名可在不改变边界的前提下遵循现有目录约定：

- `assets/inject/renderer-inject.js`：入口锚定、单实例挂载、原生导航协调和 CDP bridge host。
- 独立的本地工作区前端源码与构建入口：十模块 UI、路由、状态和 bridge client；构建产物由 CCP 打包，不在线加载。
- `crates/claude-codex-pro-core/src/routes.rs`：受控 Multica workspace/execution bridge 路由。
- Core Multica adapter：typed API client、revision/幂等、事件同步和错误模型。
- Core Codex page execution adapter：通过当前页面原生 host 完成 task/thread/subagent 创建、打开、继续、取消和事件读取；不包含 app-server transport 或 runtime registry。
- CCP 独立存储迁移：workspace 绑定、execution binding、attempt lineage、command 和 cursor。
- `apps/claude-codex-pro-manager/`：在普通设置中提供持久化工作区开关和只读诊断；独立 `Multica Runtime` 页面不进入主导航，legacy 外部连接/下载/登录/daemon 能力默认关闭且仅从高级兼容入口显式访问；不增加日常任务看板，也不提供 Codex runtime 注册。
- Rust、TypeScript、Playwright/CDP 契约测试和必要 fixture。
- LICENSE/NOTICE、用户文档和上游版本清单。

## 分阶段实施计划

### 阶段 0：契约与许可门禁

- 固定 Multica 兼容版本、API schema、发行资产摘要和许可义务。
- 建立 feature flag、typed DTO、错误码、数据迁移和 `CodexExecutionService` fake。
- 用契约测试证明 bridge 不接受任意 URL/命令，且不会触碰供应商/代理/模型配置。

### 阶段 1：页面壳与只读工作区

- 在插件下方、项目上方插入单入口，完成重注入、卸载和响应式页面壳。
- 接入 bootstrap、workspace 选择、十模块导航及各模块真实加载/空/错/过期态。
- 运行最窄的注入契约、前端类型检查和 bridge 只读测试。

### 阶段 2：Issue、项目和看板写入

- 实现 Issue/项目 CRUD、分配、父子关系、revision/CAS 和拖拽回滚。
- 增加并发冲突、重复提交、离线和权限测试。
- 暂不启动 Codex 执行，先证明 Multica 状态写入闭环。

### 阶段 3：Codex 页面原生执行闭环

- 实现预留 run、创建原生 thread、绑定、打开、继续、取消、重试和事件投影。
- 再实现智能体与小队的 parent thread/subagent 映射。
- 用 fake 页面 host 和真实 Codex 各证明一次单任务端到端；确认没有新增 Codex 子进程、app-server、npm/Claude/其他 Provider 或第二执行器，所有执行事件都来自当前页面原生 API。

### 阶段 4：自动化、统计、运行时和 Skills

- 接入自动化触发/租约/历史、审计统计、当前页面能力和受信任 Codex Skill 清单、任务/智能体/小队绑定及实际加载审计。
- 验证离线排队、租约重领、幂等恢复和小队部分失败。

### 阶段 5：最终回归与发布

- 仅在最后一轮运行 workspace 全量测试、前端完整构建、release 构建和真实 Codex 回归。
- 进行 Windows/macOS 窗口几何、重注入、主题、供应商、代理、模型选择和 Claude/Codex 启动隔离验收。
- 完成许可证、NOTICE、版本锁、回滚和升级兼容检查后才允许发布。

## 测试策略

- 每个阶段只运行覆盖该阶段改动及直接影响范围的最窄测试；失败时先修复该层，不提前反复运行全仓库回归。
- Core 使用 fake 本地 Multica 控制面和 fake Codex page host 覆盖成功、超时、401/403、409 revision、重复命令、部分提交和事件 gap。
- 注入使用静态契约和 Playwright/CDP 两层测试；实时 UI 验收必须重启或重新注入最新构建，不能用旧 renderer 状态代替。
- 数据迁移使用临时数据库，验证升级、回滚、重复执行和中断恢复。
- 真实端到端至少证明：创建 Issue、分配、唯一原生 thread、状态投影、打开同一 thread、继续、终态新 attempt 和数据保留。
- 最后一轮才执行 `npm ... check/build`、`cargo test --workspace`、`cargo build --release` 等大范围回归。

## 交付范围

- Codex 左侧入口与本地工作区页面壳。
- 十个模块及其真实 Multica 数据工作流。
- Multica typed adapter 与安全 bridge。
- Codex 原生执行适配器、映射存储、事件同步和对账。
- Manager 的持久化工作区开关、只读诊断和默认隐藏的 legacy 高级兼容入口，不包含主导航 Runtime 页面、默认下载/登录/daemon 监管、Codex runtime 注册或 app-server 管理。
- 定向测试、最终回归、Playwright/CDP 证据和真实端到端记录。
- 数据迁移、版本锁、LICENSE/NOTICE、用户文档和回滚说明。

## 回滚与恢复

- 持久化工作区开关可立即隐藏入口、卸载页面壳并停止新派发和事件订阅；应用重启后仍保持关闭，重新开启后从持久游标恢复。回滚不得删除 Multica workspace、Issue、Codex thread 或映射历史。
- UI bundle 或注入失败时回退到无工作区入口的原生 Codex；Manager 仍可诊断本地控制面，但不接管 Codex 执行。
- 新版本数据库迁移必须有版本号、事务和向后只读策略；回滚版本无法理解新字段时保留数据库并禁用写入，不破坏数据。
- 本地控制面更新失败只回滚其自身数据/资源版本，不影响 Codex、CCP 代理和供应商；不得触发 Codex runtime 注册或进程切换。
- 对已创建但未绑定的 Codex thread，恢复程序只做对账和人工确认，不自动删除；对已预留但未创建的 Multica run，按幂等状态标记失败或重试。
- 关闭集成后用户仍可在 Codex 中访问既有原生对话，并可从 Manager 导出脱敏诊断和映射清单。
