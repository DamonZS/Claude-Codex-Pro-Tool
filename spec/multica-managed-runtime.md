# Legacy 高级兼容：Multica 托管运行时与下载安装

## 规格状态与优先级

本规格保留用于已有外部 Multica CLI/daemon 用户的 **legacy 高级兼容能力**，不是 Codex 内嵌本地 Multica 工作区的默认架构，也不进入 Manager 主导航。`spec/codex-multica-workspace-integration.md` 对默认启动、页面入口和执行边界拥有更高优先级。

默认 CCP/Codex 启动路径不得创建托管连接、下载或安装 CLI、创建 profile、发起登录、启动或监管 daemon、注册 Codex Runtime，也不得启动或连接 `codex.exe app-server`。本规格后续的安装、登录和监管要求仅在用户明确进入“高级兼容”并单独启用 legacy 托管能力后适用；本地 Multica 工作区的启用/停用开关不得隐式触发这些动作。

## 背景

部分历史用户需要连接完整 Multica server/CLI/daemon。CCP 为这类用户保留一条可重复、可审查、显式启用的兼容通道：用户进入高级兼容入口并确认后，才可从固定资源或官方 GitHub Release 下载、校验和安装 Multica CLI，并选择是否监管 daemon。

托管运行时只负责 Multica CLI/daemon 的本地生命周期和只读状态连接。Multica server、数据库和远端任务仍属于独立控制平面。托管流程不能通过改写 CCP 供应商或客户端配置来实现代理，也不能把失败的安装、认证或健康检查传导成 CCP 主界面不可用。

`spec/multica-runtime-adapter.md` 中关于外部连接、只读检查、状态快照、配置隔离和手工 sidecar 的要求继续有效。所有 legacy 托管开关默认关闭；不存在覆盖“sidecar 默认关闭”的自动启动例外。

## 目标

### 本次包含

- 发布构建为受支持平台声明固定 Multica CLI 版本、目标平台、精确资产名称和 SHA-256 摘要。
- 优先从 CCP 发布包资源目录取得对应平台资源；资源缺失或损坏时，只从该固定版本的官方 GitHub Release 下载对应 allowlist asset。
- 对下载内容和发布包资源执行 SHA-256 校验、归档安全检查、临时文件安装、原子激活、并发锁和失败回滚。
- 用户显式启用 legacy 托管能力后，才幂等创建稳定连接 ID `managed-multica` 和独立 profile `ccp-managed`；默认 `enabled/auto_start/supervise=false`。
- legacy 高级兼容入口不出现在 Manager 主导航，不是本地工作区的默认主数据源或运行前置条件。
- 提供安装状态、版本、来源、校验结果、登录、启动/停止/重启和有限重试的 Tauri/Manager 操作。
- 托管 CLI 使用独立 profile 目录和进程组；其 URL、认证、日志、端口、生命周期与 CCP 供应商、代理、Codex、Claude 配置隔离。
- 为版本选择、下载校验、原子安装、回滚、显式连接初始化、profile 隔离、监管和 URL 不改写增加最小精度测试，并在最后一轮执行完整回归验收。

### 本次不包含

- 不打包或自动安装 Multica server、PostgreSQL、Redis、Web/Electron UI，不迁移远端数据库。
- 不实现 Multica Task 创建、领取、取消、重试、消息发送、进度写入、Review 或 Agent 调度。
- 不将 Multica Provider/Runtime 映射为 CCP 供应商、Profile、active profile、模型或上游 URL。
- 不调用 `switch_relay_profile_in_home`，不修改或重写 `config.toml`、`auth.json`、Claude 配置、Codex 配置、供应商记录、CCP 代理配置或 API Key。
- 不使用用户输入的任意下载 URL、脚本、shell 命令、通用进程名或未验证的可执行文件作为托管运行时。
- 不改变现有供应商路由、协议代理、代理端口 `57321`/`57331`/`57320`、Codex/Claude 启动和注入行为。
- 不强制覆盖用户已经保存的托管连接 URL、显示名称、profile 配置、禁用状态或登录状态；自动默认值仅用于不存在托管记录的首次引导。

## 用户视角描述

用户正常安装或启动 CCP 时不会检查、下载或安装 `multica.exe`。只有用户进入 legacy 高级兼容入口、开启该能力并确认“准备/安装”后，CCP 才检查内置资源或从固定官方 Release 获取、校验和安装。

legacy 首次引导在用户确认后创建“内置 Multica Runtime”连接，使用 profile `ccp-managed`，自动启动和监管默认关闭。用户可在高级兼容面板查看安装来源、版本、校验状态、daemon PID、运行状态、最近检查时间和登录状态。没有 token 时显示“需要登录/未授权”，不会把进程存在当作服务健康。

高级兼容面板只在用户明确打开时读取托管连接及其 Server、daemon、Runtime、Agent、Task 状态。面板不存在或未启用不得影响本地 Multica 工作区入口、数据或 Codex 原生执行。

普通外部连接是兼容旧适配器的可选高级功能，与托管能力在同一 legacy 高级兼容入口内分区展示。用户明确进入或选择外部连接后，才查看对应外部连接状态；外部连接的新增、选择、编辑、sidecar 和快照状态不得替代或清空本地工作区状态。

用户可以修改 server URL、显示名称、监管开关和登录状态。保存后启动、刷新和重启均使用用户保存的原值；清空字段不会在每次保存或刷新时被偷偷回填或归一化。用户停用托管连接后，CCP 不再启动或重启该 daemon，但不会删除 profile、远端数据或任何供应商配置。

## 功能要求

### 1. 固定版本与平台资源清单

- 构建配置必须声明唯一的 `multica_version`、目标平台/架构、精确 `asset_name`、归档格式和期望 SHA-256。版本不能在运行时通过“latest”或用户输入改变。
- 每个支持的目标三元组使用独立 allowlist 条目，例如 `x86_64-pc-windows-msvc`、`x86_64-apple-darwin`、`aarch64-apple-darwin`；无法匹配条目时状态为 `unsupported_platform`，不得下载相近架构资产。
- 发布包中的资源必须带有同一版本和平台元数据。内置资源存在时先校验再使用；摘要不匹配视为损坏，不得绕过校验运行。
- 已安装目录只能激活 allowlist 中的固定版本。发现其他版本时保留为历史文件，不能静默切换到未经当前构建声明的版本。
- 资产清单、版本和摘要属于构建产物的一部分，发布/CI 必须检查资产名、平台和摘要彼此一致；不得把密钥、token 或完整授权材料编入资源。

### 2. 官方 Release 下载

- 只有在没有可用的已安装固定版本且内置资源缺失或损坏时才下载。下载地址由固定版本、固定 asset name 和官方仓库常量拼接，禁止任意 URL、重定向到非 allowlist 主机或自动跟随“最新版本”。
- 仅允许 HTTPS；初始主机必须是官方 GitHub Release 主机，有限重定向只能落到该 Release 的官方资产 CDN allowlist。超出主机、协议或重定向次数上限立即失败。
- 同时取得该版本 Release 的 `checksums.txt`（或构建声明的等价校验清单），按完整文件名解析 SHA-256，不能用模糊字符串匹配。校验清单来源必须与固定版本和官方仓库绑定。
- 下载使用连接超时、总超时、大小上限、取消令牌和最多 3 次有限尝试；只有网络错误、超时、HTTP 408/425/429/500/502/503/504 可以退避重试，永久 4xx、重定向拒绝、校验失败和取消不得重试。
- 下载进度只报告字节数、总大小、版本、平台和脱敏错误码。响应正文、请求头、Cookie、Authorization 和 token 不得写入日志或前端。
- 网络不可用、校验清单缺失、HTTP 非 2xx、TLS/DNS 错误或内容超限时保留现有可用版本，状态标为 `download_failed`/`verification_failed`，不阻塞 CCP 其他功能。

### 3. 校验、临时安装与回滚

- 下载和资源复制必须写入应用专属临时目录，临时文件名包含随机标识；文件关闭并完成 flush/sync 后才进行 SHA-256、归档格式和入口文件检查。
- 解压必须拒绝绝对路径、`..` 穿越、符号链接逃逸、重复覆盖和超出文件/总大小上限的归档；只允许预期的 CLI 入口文件和必要资源落入版本目录。
- 安装目录使用版本化子目录和原子激活指针/元数据。激活前取得跨进程安装锁，不能让两个 CCP 实例同时写同一版本目录。
- 激活顺序必须是：下载/复制到临时目录 → 摘要和归档检查 → 安全解压 → 校验入口可执行文件 → 执行无副作用的 `--version`/能力探测 → 原子切换当前版本。任一步失败都不得留下半成品当前版本。
- 切换前保留上一个可用版本和其摘要。只有可归因于当前托管二进制的启动失败、版本输出不匹配、daemon 健康探测失败或进程异常退出，才自动恢复上一个可用版本；空/非法 Server URL、连接停用、应用退出中、托管 sidecar 固定契约错误等配置或生命周期拒绝只返回诊断，不得切换当前/上一版本指针。无旧版本时状态为 `unavailable`，但 CCP 继续启动。
- 安装、升级、损坏重装或回滚切换当前版本指针时，必须把托管连接内部 sidecar 的 `executable`、固定 profile 工作目录、固定参数和 `auto_start` 整体同步到当前托管契约；即使 executable 路径未变化也要修复旧参数。该同步不得改写显示名称、Server URL、enabled 或任何普通连接。
- 回滚只修改托管运行时自己的安装指针、临时文件和状态缓存；不得回滚或覆盖 CCP 供应商、代理、Codex/Claude 文件。临时目录清理必须限定在应用专属目录。
- 安装、升级、回滚和取消操作在 UI 中可观察，应用重启后能够从原子元数据恢复当前/上一版本，不因崩溃而重复破坏安装。
- 安装、升级或显式回滚遇到正在运行的托管 daemon 时，必须先持有安装锁并验证进程归属，再停止该进程树、切换原子指针与 sidecar 固定契约；切换前原本在运行的，切换后才重新启动。外来 PID 或无法核验可执行路径时拒绝切换，不得边运行边替换二进制。
- 最近一次安装失败写入托管状态目录的 `install-failure.json`，只允许保存稳定错误码和时间戳；不得保存下载 URL、响应、文件路径、token 或原始错误。已有当前版本仍可用时，失败记录不能覆盖 `ready`/校验状态；后续成功安装必须清除旧失败记录。

### 4. 托管连接与 profile 初始值

- 用户显式启用 legacy 托管能力且没有 `managed-multica` 记录时，才创建：
  - `connection_id`: `managed-multica`
  - `display_name`: `内置 Multica Runtime`
  - `server_url`: `https://api.multica.ai`
  - `profile`: `ccp-managed`
  - `enabled`: `false`（必须由用户再次明确开启连接）
  - `auto_start`: `false`
  - `supervise`: `false`
- 上述默认值只在首次创建缺失字段时使用。用户后续修改或清空显示名称、URL、开关时，保存、刷新、安装和重启不得自动回填、归一化、改大小写、补尾斜杠或改成 CCP 代理地址。
- `ccp-managed` profile 必须位于 Multica 默认 profile 根目录下的独立目录（Windows 为 `%USERPROFILE%\.multica\profiles\ccp-managed\`，其他平台使用对应用户目录），与用户其他 profile 分离。已存在的 profile 配置只读保留，除非用户明确触发对应的登录/配置动作。
- 托管连接记录和安装元数据存储在 CCP 独立设置命名空间，不能复用供应商/Profile 表或让供应商写入逻辑处理它。
- 同一 `managed-multica` 记录必须幂等初始化；并发启动不能创建第二个托管连接或覆盖用户已保存的值。

### 5. 显式准备、启动与监管

- legacy 功能未由用户显式启用时，CCP 启动只读取关闭状态，不执行版本检查、安装、下载、profile 创建、登录或 daemon 启动。准备流程只能由高级兼容面板中的用户动作触发，且不得阻塞窗口、供应商页面、代理监听或 Codex/Claude 启动。
- 只有 legacy 功能已显式启用、用户又单独保存 `enabled=true`、`auto_start=true`、`supervise=true`，并且运行时和 profile 已由显式准备动作验证可用时，后续启动才可恢复 `multica --profile ccp-managed daemon start --foreground`；本地工作区开关不参与该判断。
- 托管 daemon 必须显式关闭 Multica 自更新和配置热重载；固定 `v0.4.36` 的启动参数必须严格为 `--profile ccp-managed daemon start --foreground --no-auto-update --no-auto-reload`。两个禁用开关属于 `daemon start` 子命令，不得放到 root flags 中，也不得由 daemon 静默替换当前 allowlist 版本或重新加载非托管配置。
- 启动前必须校验托管连接保存的 `server_url`。仅在托管子进程环境中先移除继承的 `MULTICA_SERVER_URL`，再注入该连接已保存且通过校验的原值；不得修改父进程、系统环境、普通 sidecar、供应商、Profile 或代理配置。显式保存的空值继续保留，并使需要 URL 的运行操作以可诊断错误失败，不得回退到默认地址。
- 托管进程必须使用独立日志、临时目录、环境变量命名空间和健康端口；不得使用 CCP Launcher/helper/watchdog，也不得绑定或改写 `57321`、`57331`、`57320`、`9230`。
- 监管记录 PID、进程树归属、启动时间、当前固定版本、profile、退出码、最近健康状态和连接 ID。停止/重启只允许作用于已验证的 PID、可执行文件路径和 profile 标识，不得按进程名杀进程。
- daemon 崩溃后仅在 `supervise=true` 时按有限次数和退避策略重启；达到上限标记 `crashed`/`restart_exhausted`，等待用户明确重试，不得无限循环。下载、安装或认证失败同样不得循环重试。
- CCP 退出时只清理本次由 CCP 启动且归属校验通过的托管进程树；外部启动的同名 Multica 进程、Codex/Claude、代理和其他用户进程不受影响。应用异常退出后，下一次启动只能在 PID、路径、profile 和版本均匹配时恢复监管。
- 新实例取得独占生命周期锁后才能处理陈旧 `owner.json`：记录 PID 已退出时仅删记录；PID 仍存在且可执行路径与记录匹配时终止该进程树并有界等待退出后删记录；路径不匹配时视为 PID 复用，只删陈旧记录且不得终止进程；路径无法核验时保留记录并拒绝接管。
- 进程存在不能单独判为 `healthy`。必须通过独立只读 health/version/status 能力探测；探测失败时状态为 `degraded`、`unreachable`、`unauthorized` 或 `invalid_response`。
- 监管线程每 5 秒对运行中的托管 daemon 执行一次有界只读 health 探测，不能长期复用启动时缓存。`unreachable`、普通 `degraded` 和 `invalid_response` 进入同一有限恢复预算；PID 归属不可信时停止监管且不杀进程；401/403 映射的 `unauthorized` 及 `needs_login` 只更新状态并继续探测，不重启 daemon。

### 6. 登录与认证

- legacy 高级兼容面板可提供固定 profile 的官方 CLI 登录/退出入口；普通设置和本地工作区页面不显示或自动调用该入口。命令由后端按已安装版本能力映射生成，不接受前端任意可执行文件、shell 片段或参数数组。
- token 使用现有安全存储或受保护的 stdin/IPC/环境引用传递；不得出现在 argv、普通环境快照、崩溃转储、URL、CCP 配置、日志、错误正文或前端状态。
- 登录成功只更新 `ccp-managed` profile 的认证状态。没有 token 或登录返回未授权时，显示 `unauthorized`/`needs_login`，不把 daemon 进程存在标成健康，也不触发无限重启。
- 退出登录/清空凭据只删除该 profile 的凭据引用；不得删除其他 Multica profile、供应商 API Key 或 Claude/Codex 认证。

### 7. 健康与只读状态

- Server 健康检查保存用户的 `server_url` 原值，使用固定版本声明的只读 health/version 端点；不调用创建任务、刷新租约或改变远端状态的接口。
- Runtime、Agent、Task 只读快照沿用 `spec/multica-runtime-adapter.md` 的分页、有界数量、`fetched_at`、`stale`、未知状态兼容和脱敏规则。安装状态另含 `install_state`、`installed_version`、`asset_source`、`sha256_verified` 和 `last_install_error_code`。
- 托管状态与托管 Runtime、Agent、Task 快照必须通过固定 `managed-multica` 的专用只读后端路径取得，前端不传 `connection_id`，也不依赖普通连接列表或普通连接选择状态。专用路径可以在后端复用只读快照实现，但不得解除普通连接 IPC 对保留 ID `managed-multica` 的拒绝。
- 用户打开 legacy 高级兼容面板时读取托管状态和最近可用托管快照；用户点击托管“刷新状态/刷新快照”后重新读取固定托管连接的数据。刷新结果必须携带 `source_connection_id=managed-multica`，不能因当前选中的普通外部连接而改变来源。
- legacy 自动恢复监管只接受用户已显式开启全部 legacy 生命周期开关、`install_state=ready` 且 `sha256_verified=true`；历史 `last_install_error_code` 不得阻止仍然有效的当前版本启动。准备失败的 IPC 必须保留 Runtime、连接、登录和 daemon 脱敏 payload，不能用空失败对象覆盖诊断。
- 状态枚举至少包括 `not_installed`、`checking`、`installing`、`ready`、`healthy`、`degraded`、`unreachable`、`unauthorized`、`download_failed`、`verification_failed`、`unsupported_platform`、`crashed`、`restart_exhausted`、`stopped`、`unconfigured`。
- 刷新失败保留上次成功快照并标记过期；安装或 daemon 不可用时不生成示例任务、虚假版本、虚假计数或“系统已接受”等误导性状态。

### 8. UI / 交互要求

- Manager 主导航不得增加或保留独立 `Multica Runtime` 页面。普通设置保留“启用 Multica 工作区”开关；该开关与本 legacy 面板的开关严格分离。
- legacy 托管 UI 只能放在明确标记为“高级兼容/Legacy”的次级入口或可折叠面板中，默认关闭且不自动加载网络或进程状态。
- 用户打开该面板后，托管区域可展示 Server/daemon 状态以及 Runtime、Agent、Task 只读快照；它不是本地工作区的默认数据源。
- 托管卡片显示：连接名称、保存的 server URL、profile 名称、安装版本/来源、校验状态、准备进度、daemon 状态/PID、Server 状态、登录状态、最近检查时间和脱敏诊断。
- 托管卡片提供独立编辑表单，且只能提交 `display_name`、`server_url` 与 `enabled` 给 `save_multica_managed_connection`。显示名称与 server URL 都允许显式保存为空字符串，表单、保存、刷新、安装和重启不得回填、归一化或改写它们；该表单不得调用普通连接 CRUD。
- 托管连接隐藏“选择任意 executable/工作目录/启动参数”字段；手工连接仍显示旧字段并保持兼容。托管操作按钮包括“准备/重试下载”“登录”“测试连接”“刷新”“启动”“停止”“重启”“回滚”，均有忙碌、成功、失败、禁用和取消状态。
- 安装、回滚、停止和删除需确认，并明确说明只影响托管运行时/该连接，不影响供应商、代理、Codex/Claude。清空 token 输入表示删除该 profile 的凭据引用，不自动回填旧值。
- `unconfigured`、`installing`、`unauthorized`、`stale`、`restart_exhausted` 等状态必须同时显示文字和无障碍语义，不能只靠颜色；网络失败时提供可操作的重试入口。
- 普通外部连接必须标记为“外部连接（可选）”或等价的高级功能，并与托管区域分区。没有普通外部连接时可以显示非阻断提示“未添加外部连接（可选）”，但不得显示“尚未配置 Multica 连接”、要求先选择连接，或隐藏/禁用托管快照；外部连接的“刷新连接”“新增连接”等操作只出现在该可选分区。
- 托管快照不可用时，在托管区域显示对应的未授权、未启动、不可达、过期或脱敏错误状态，并保留可用的刷新/登录/启动动作；不得退回普通连接空态或生成示例数据。

### 9. Tauri/Core 接口

Core 层提供独立模型和服务，至少包含：

```rust
MulticaManagedRuntimeConfig
MulticaRuntimeInstallStatus
MulticaManagedConnection
MulticaDaemonStatus
MulticaRuntimeSnapshot
```

Tauri 命令边界至少包括：

```text
get_multica_managed_runtime
get_multica_managed_snapshot
ensure_multica_runtime
cancel_multica_runtime_install
rollback_multica_runtime
login_multica_managed
logout_multica_managed
set_multica_managed_enabled
save_multica_managed_connection
check_multica_connection
get_multica_snapshot
start_multica_sidecar
stop_multica_sidecar
restart_multica_sidecar
check_multica_managed_runtime
start_multica_managed_runtime
stop_multica_managed_runtime
restart_multica_managed_runtime
```

接口约束：

- 托管准备命令不接受任意 URL、可执行文件、PID、环境变量或请求头；所有值由后端固定清单、已保存连接 ID 和已验证安装元数据决定。
- `save_multica_managed_connection` 只接受 `display_name`、`server_url` 和 `enabled`，不接受连接 ID、profile、sidecar、可执行文件、路径、令牌、请求头或普通连接字段；它只更新固定的 `managed-multica` 记录。
- `check_multica_connection`、`get_multica_snapshot` 和状态查询只读；安装命令只写应用专属安装目录、托管 profile 所需目录和 CCP 独立状态，不写供应商/客户端文件。
- `check_multica_managed_runtime` 与托管生命周期命令不接受连接 ID；它们只作用于固定的 `managed-multica` 记录。普通 `check_multica_connection`、`get_multica_snapshot`、`start_multica_sidecar`、`stop_multica_sidecar` 和 `restart_multica_sidecar` 必须拒绝该保留 ID。
- `get_multica_managed_snapshot` 不接受连接 ID，只读取固定 `managed-multica` 的 Runtime、Agent、Task 快照并返回 `source_connection_id=managed-multica`。如选择把快照并入 `get_multica_managed_runtime` 的 payload，也必须保持同一固定来源、只读语义和普通 IPC 拒绝边界。
- 返回 DTO 只包含版本、平台、来源、状态、时间、耗时、错误码和脱敏摘要；不包含 token、Authorization、Cookie、完整响应正文、完整本机路径或任意原始 argv。
- 所有异步操作支持取消、幂等和有限超时；重复准备请求复用进行中的任务或取消旧任务，不能并行安装同一版本。

### 10. 配置与供应商/代理隔离

- 启动、安装、登录、健康检查、刷新、停止和回滚前后，供应商列表、显示名称、active Profile、上游 URL、`config.toml`、`auth.json`、Claude 配置和 CCP 代理监听状态必须保持不变。
- Multica 的 `server_url` 永远是 Multica 连接自己的字段。任何流程都不得把它或任何供应商 URL 改成 `http://127.0.0.1:57321/v1`、`http://127.0.0.1:57331/v1` 或其他 CCP 本地代理地址。
- Multica 若需要经过代理，必须显式读取系统/Multica 连接代理设置；不得把 CCP 供应商路由、active profile 或协议转换配置注入 Multica profile。
- 适配器的网络、安装、监管异常只能更新独立状态并提示用户，不能阻塞主界面或改变 Codex/Claude 启动结果。

### 11. 日志、权限与故障处理

- 日志仅记录动作、连接 ID、版本、平台、资产名（非敏感）、状态、阶段、耗时和脱敏错误码；禁止 token、Authorization、Cookie、完整请求 URL、原始响应、工作区绝对路径和完整命令行。
- Windows 下下载、解压、daemon 和登录命令无控制台窗口启动；其他平台保持无头语义。安装目录权限遵循用户级最小权限，不要求管理员权限。
- 安装锁、下载取消、应用崩溃、磁盘不足、权限拒绝、TLS/DNS、校验失败、CLI 版本不匹配和 daemon 退出必须分别分类，向用户提供下一步操作。
- 任何阶段失败都保留当前可用版本和最近诊断；首次准备失败时以“未配置/不可用 Multica”模式启动 CCP，供应商、代理和 Codex/Claude 继续可用。

## 技术约束

- 托管运行时位于 core 独立模块；Tauri commands 负责 IPC、权限和结果包装，React 只消费脱敏 DTO。
- 复用现有 HTTP 客户端、设置原子写入、日志和平台进程/作业对象能力；不引入完整 Multica server、数据库或不必要的包管理器。
- 资源清单、下载器和安装器必须可在离线环境测试；网络测试使用本地 fake server/fixture，不依赖真实 token 或生产服务。
- 归档解析采用结构化库和路径校验，禁止用字符串拼接执行 shell。所有外部进程使用显式参数数组、已验证路径和独立环境。
- Windows/macOS 的安装路径、进程树回收、无窗口启动和文件替换必须分别覆盖；平台不支持时返回 `unsupported_platform`，不能回退到错误架构。
- 任何供应商、代理、Codex/Claude 代码改动必须有独立需求和 spec；本任务默认禁止修改这些边界。

## 测试策略

- 开发过程采用最小精度测试：每次代码变更只运行直接覆盖变更逻辑及其直接影响面的测试。例如清单解析只跑版本/平台/allowlist 测试，安装器变更只跑 hash、原子替换、回滚和并发锁测试，监管变更只跑 PID 归属、健康分类和有限重启测试，UI 变更只跑相关类型/组件检查。
- 只在变更触及共享命令包装、设置写入、进程管理或供应商隔离边界时，增加对应的直接调用方测试；不得在每个迭代用整仓库回归掩盖失败原因。
- 最后一轮才执行大范围回归：完整 workspace Rust 测试、Manager 类型检查和 Vite 构建、Release 构建、安装包资源检查、托管运行时现场 smoke test，以及供应商 URL/代理/Codex/Claude 启停未被改变的回归证据。
- 每一轮都记录实际命令、退出码和失败归因；不得把未执行的测试写成通过。真实网络、真实登录和真实发布资产只用于最后一轮或明确的现场 smoke test，测试凭据必须脱敏并在结束后清理。

## 交付范围

- 本规格及匹配的 `acceptance/multica-managed-runtime.md`。
- Core 托管运行时清单、下载、校验、安装、回滚、profile/连接初始化、登录和 daemon 监管实现。
- Tauri 命令注册、脱敏状态 DTO、默认隐藏的 Manager legacy 高级兼容面板及显式安装/登录/监管交互；不交付主导航 Runtime 页面。
- 发布配置和 CI 检查：固定版本/平台资产、校验清单、资源打包、离线缺失资源下载路径和默认 `target/release` 产物。
- 最小精度定向测试、最后一轮大回归测试和本地 smoke 证据；不交付 Multica server、PostgreSQL、Redis 或其迁移。

## 回滚与恢复

- 下载或校验失败：删除本次临时文件，保留当前/上一可用版本，托管状态标记失败；不触碰其他配置。
- 安装激活或首启健康失败：原子恢复上一版本和旧元数据，停止只属于新版本的进程树，保留脱敏诊断。
- 托管运行时初始化或配置迁移失败：CCP 以未配置 Multica 模式继续启动。删除/停用托管连接只删除独立记录和状态缓存，不删除远端数据、用户其他 profile、供应商或客户端配置。
- 用户明确点击回滚时，只能回到安装元数据中已验证的上一版本；没有可用历史版本时显示不可回滚，不下载未知版本替代。
