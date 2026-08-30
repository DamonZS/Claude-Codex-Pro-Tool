# Multica Runtime Adapter 首个增量

## 背景

Claude Codex Pro Tool（CCP）负责本机供应商、代理、Codex/Claude 客户端和启动器生命周期；Multica 负责 Issue、Agent、Task、Runtime、租约和协作状态。两者属于不同控制平面，不能通过互相覆盖配置来实现整合。当前需要先建立一条可审查、可关闭、不会改变 CCP 现有代理和客户端行为的连接边界，为后续任务执行适配器提供运行时基础。

本规格定义首个可交付增量：在 CCP 管理工具中配置一个外部 Multica server/daemon 连接，执行只读健康检查和状态同步；在用户明确启用时，可由 CCP 监管一个独立的 Multica daemon sidecar。首个增量不下发或执行任务，不改变供应商/Profile，不写入 Codex/Claude 全局配置。

## 目标

本次包含：

- 保存、编辑、删除和启用/停用外部 Multica server 连接配置。
- 对 Multica server API 和 daemon 分别执行有界超时的只读健康检查。
- 可选监管用户明确指定的 Multica daemon sidecar：启动、停止、重启、进程存活和退出状态观察。
- 在管理工具中只读展示 Multica 连接、daemon、Runtime、Agent 和 Task 的脱敏状态摘要。
- 提供连接测试、刷新、失败诊断、最近检查时间和状态来源，确保“系统接受请求”不被误报为“服务可用”。
- 为配置隔离、健康检查、sidecar 生命周期和只读数据映射增加可测试契约。

本次不包含：

- 不实现 Multica Task 创建、领取、取消、重试、消息发送、进度写入或 Review 操作。
- 不把 Multica 的 Provider/Runtime 字段映射为 CCP 供应商名称、Profile ID、active profile 或上游 URL。
- 不调用 `switch_relay_profile_in_home`，不写入或重写 `config.toml`、`auth.json`、Claude 配置、CCP 设置或任何 API Key。
- 不把任何供应商上游地址改写为 `http://127.0.0.1:57321/v1`、`http://127.0.0.1:57331/v1` 或其他 CCP 固定代理地址。
- 不复用 Codex/Claude GUI 启动器、helper、watchdog、注入脚本或其固定端口；不向 Codex/Claude GUI 注入 Multica UI 或模型选项。
- 不把 Multica server、PostgreSQL、Redis 或 Web UI 合并进 Tauri 进程；不迁移或复制 Multica 数据库到 CCP SQLite。
- 不改变现有供应商路由、代理转发、Codex/Claude 启动和注入行为。

## 用户视角描述

用户在管理工具的“设置/运行时”区域添加 Multica 连接，填写服务地址和可选访问令牌，点击“测试连接”即可看到服务版本、API 可达性和 daemon 状态。令牌在界面中默认隐藏，日志和状态列表永不显示原文。

用户可以选择“由 CCP 监管本机 Multica daemon”，指定已安装 daemon 的可执行文件和工作目录。启用后，CCP 只负责该 sidecar 的独立进程生命周期和健康状态；停止或退出管理工具时不会终止 Codex/Claude GUI、CCP 代理或其他用户进程。

连接成功后，页面以只读列表显示最近同步的 Runtime、Agent 和 Task 摘要，包括状态、更新时间、任务标题（按服务返回的安全长度截断）和错误摘要。列表刷新失败时保留上次成功快照并标注过期，不得伪造实时状态。页面没有创建、编辑、删除或执行 Multica 任务的入口。

## 功能要求

### 1. 连接配置

- 连接配置至少包含：稳定 `connection_id`、显示名称、HTTPS 或明确允许的本机 HTTP `server_url`、可选 `api_prefix`、令牌引用、启用状态、创建/更新时间。
- `server_url` 保存为 Multica 上游地址原值；保存、读取、刷新和健康检查均不得经过 CCP 供应商 URL 归一化或代理地址替换。
- 默认只允许 `https://`；`http://` 仅允许 loopback 或用户明确确认的局域网地址，并在 UI 标出非加密风险。禁止 `file://`、`javascript:`、嵌套凭据 URL 和隐式重定向到非允许协议。
- 令牌必须使用现有安全存储或受保护本地凭据引用；配置导出、日志、错误和只读列表只能返回是否已配置，不返回令牌原文。
- 同一 `server_url` 不得因尾斜杠、大小写或 API 前缀重复生成多个隐式连接；去重只用于连接记录，不得修改用户保存的显示名称或 URL。
- 删除连接前若 sidecar 仍由该连接监管，必须先停止 sidecar 或明确拒绝删除；删除不得触碰 Multica 数据库、CCP 供应商或客户端配置。

### 2. 只读健康检查

- Server 检查使用 Multica 明确的只读健康/version 端点；优先 `GET`/`HEAD`，不得用会创建任务、刷新租约或改变状态的接口作为健康检查。
- Daemon 检查通过独立 health 端点或本地状态 RPC；不得把“进程存在”单独判定为 daemon 可用。
- 每次检查使用连接级取消令牌、连接超时和总超时；超时、DNS、TLS、HTTP 非 2xx、JSON 不兼容和认证失败必须分别归类。
- 必须区分 `unconfigured`、`checking`、`healthy`、`degraded`、`unreachable`、`unauthorized`、`invalid_response` 和 `stopped`，不得以 HTTP 请求发出或命令退出码为零代替服务就绪。
- 检查结果至少包含：连接 ID、服务/daemon 状态、探测端点类型、HTTP 状态（如有）、服务版本（如有）、检查时间、耗时、脱敏诊断和数据新鲜度。
- 健康检查不能启动或停止 Codex/Claude、不能切换 active relay profile、不能启动 CCP 代理，也不能写入任何供应商或客户端文件。

### 3. 可选 sidecar 监管

- sidecar 监管默认关闭，只有用户显式启用并确认可执行文件、工作目录和参数后才允许启动。
- 可执行文件路径必须是用户选择或已验证的本地文件；不得下载、自动安装、执行 shell 脚本或拼接未经验证的命令字符串。
- sidecar 使用独立的进程组/作业对象、环境变量命名空间、日志路径和健康检查端口；不得复用 CCP Launcher 的 helper/watchdog 进程或端口 `57321`、`57331`、`57320`、`9230`。
- 传递给 sidecar 的参数中不得出现 API Key、Bearer Token 或完整凭据；令牌通过受保护环境引用或 IPC 注入，且不得出现在 argv、崩溃转储和普通日志。
- CCP 必须记录 sidecar PID、启动时间、退出码、最近健康状态和监管连接 ID；sidecar 崩溃后只更新状态并按用户设置有限重启，不得无限重启。
- 停止、重启和退出清理只作用于该 sidecar 的进程树；必须验证目标可执行文件路径和 PID 归属，禁止按通用进程名杀进程。
- sidecar 启动失败、健康检查失败或重启次数耗尽时，Multica 状态标记为不可用，但 CCP 供应商、代理、Codex/Claude GUI 必须继续可用。

### 4. 只读状态同步与展示

- 状态同步仅调用 Multica 只读查询接口，至少支持 Runtime、Agent、Task 的分页或有界数量读取；不得自动遍历无限历史。
- 前端展示统一脱敏模型：稳定 ID、名称/标题、状态、更新时间、Runtime 类型、错误摘要和来源连接；不展示令牌、Authorization、完整请求 URL、工作区绝对路径或原始消息正文。
- 状态快照必须带 `fetched_at`、`source_connection_id` 和 `stale` 标记。刷新失败时保留最近成功快照，明确显示“数据可能已过期”。首次加载失败时显示错误空态，不生成示例任务或虚假计数。
- Multica 返回未知状态、缺失字段或新增字段时，解析器不得丢弃整个列表；未知状态映射为 `unknown` 并保留可读诊断。
- 列表操作只能刷新、展开脱敏详情和复制稳定 ID；不提供执行、取消、重试、编辑或删除动作。
- CCP 关闭连接或停用 sidecar 后，已缓存状态可以保留为历史快照，但必须标注来源不可达，不得继续显示为实时健康。

### 5. 并发与故障隔离

- 同一连接的健康检查和状态刷新必须去重或取消旧请求，避免重复轮询；不同连接可并行但必须受总并发上限约束。
- Multica server 不可达、返回异常数据或 sidecar 崩溃不得阻塞 CCP 主界面、供应商页面、代理服务或 Codex/Claude 启动。
- 所有错误提示使用中文、可操作且脱敏；网络响应正文只保留有限长度摘要，禁止写入令牌和完整请求头。
- 应用重启后应恢复连接配置和停用状态；sidecar 是否自动恢复必须由独立开关控制，默认不自动恢复。

## UI / 交互要求

- 在现有管理工具中增加独立的“Multica Runtime”区域，不改变供应商、Codex、Claude 和代理页面布局及交互。
- 连接卡片显示名称、地址（可隐藏路径部分）、启用状态、Server 健康、Daemon 健康、最后检查时间和“测试连接/刷新”按钮。
- sidecar 监管使用独立开关和启动/停止/重启按钮；按钮必须显示忙碌、成功、失败和禁用状态，不能与 Codex/Claude 启停按钮混用。
- Runtime、Agent、Task 使用只读标签页或列表；分类数量来自最近快照，列表加载和过期状态必须可见。
- `healthy`、`degraded`、`unreachable`、`unauthorized`、`stale` 等状态应有明确文字和无障碍语义，不能只用颜色区分。
- 连接编辑表单保存前显示协议、主机和 sidecar 可执行文件摘要；令牌输入默认为密码控件，清空令牌表示删除引用，不得自动回填旧值。
- 删除、停用和停止 sidecar 需要确认；确认文案必须说明仅影响 Multica 连接或该 sidecar，不影响 CCP 供应商、代理和 Codex/Claude。

## 数据与接口要求

Core 层新增独立的 Multica 连接与状态模型，建议包含：

```rust
MulticaConnectionConfig
MulticaConnectionStatus
MulticaDaemonStatus
MulticaRuntimeSnapshot
MulticaAgentSnapshot
MulticaTaskSnapshot
```

Tauri 命令边界至少包括：

```text
list_multica_connections
save_multica_connection
delete_multica_connection
check_multica_connection
get_multica_snapshot
start_multica_sidecar
stop_multica_sidecar
restart_multica_sidecar
```

接口约束：

- `check_multica_connection`、`get_multica_snapshot` 和所有列表接口必须是只读；调用它们不得修改 CCP 设置、供应商、Profile、客户端配置或 Multica 任务状态。
- 前端不得传入任意文件路径、PID、环境变量或上游请求头覆盖；后端根据已保存连接 ID 和已验证 sidecar 配置执行。
- 所有返回结构使用稳定枚举和可选字段，未知字段向前兼容；分页、数量上限和时间范围由后端强制执行。
- 日志只记录动作、连接 ID、端点类别、状态、耗时和脱敏错误码；不得记录令牌、Authorization、Cookie、原始响应正文或 API 请求正文。
- 配置写入沿用 CCP 现有原子写入与备份策略，存储位置独立于供应商/Profile 设置；迁移或写入失败时保留原配置并返回明确错误。

## 技术约束

- 适配器应位于 core 独立模块，Tauri commands 只负责 IPC、权限边界和结果包装；React 只消费脱敏 DTO。
- 复用现有 HTTP 客户端、日志和设置原子写入能力，不新增不必要依赖；sidecar 进程管理使用平台原生进程/作业对象能力。
- Multica server、daemon、PostgreSQL、Redis 均视为外部依赖；CCP 不内嵌数据库、不接管其迁移、不修改其数据目录。
- Windows 下 sidecar、健康检查和后台 CLI 必须无控制台窗口启动；macOS/Linux 保持无头执行语义。
- 任何代理请求必须显式使用 Multica 连接配置或系统代理；不得把 CCP 本地代理地址注入 Multica 配置，也不得让 Multica 改写 CCP 上游 URL。
- 适配器故障隔离在独立任务和状态缓存中；不得阻塞现有 Tauri 命令线程或改变 GUI 启动器生命周期。
- 实现前必须补充匹配的 `acceptance/multica-runtime-adapter.md`，并覆盖配置隔离、只读检查、sidecar 进程归属、脱敏和回归验证。

## 交付范围

- 本规格及匹配验收文档。
- Core 层 Multica 连接配置、HTTP 健康检查、状态快照和 sidecar 监管模块。
- Tauri 配置、检查、快照和 sidecar 生命周期命令及注册。
- Manager 的 Multica Runtime 配置页、健康状态和 Runtime/Agent/Task 只读列表。
- 定向 Rust/前端测试，覆盖 URL 不改写、全局配置哈希不变、超时/错误分类、sidecar PID 归属、停止不误杀和敏感字段脱敏。
- 默认 `target/release` 构建及本地手动验证证据；不交付 Multica server、PostgreSQL 或 Redis 的内嵌打包。

## 回滚与恢复

- 删除或停用适配器只删除其独立连接记录和本地状态缓存，不删除 Multica 远端数据，不触碰 CCP 供应商/客户端配置。
- sidecar 启动前保存监管状态；停止或崩溃后清理该 sidecar 的临时 PID/日志引用，保留诊断快照。
- 适配器初始化失败、配置迁移失败或升级回滚时，CCP 必须继续以未配置 Multica 的模式启动，现有代理和 GUI 功能不受影响。
