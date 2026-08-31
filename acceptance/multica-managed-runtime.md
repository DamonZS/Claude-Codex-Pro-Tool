# 验收标准：Legacy 高级兼容 Multica 托管运行时

验证对象：`spec/multica-managed-runtime.md`

本验收采用两阶段测试策略：开发过程只运行与当前变更直接相关的最小精度测试；全部功能冻结后才执行最后一轮大范围回归。任何未实际执行的命令不得记为通过。

本验收仅在用户显式启用 legacy 高级兼容能力后适用。默认 CCP/Codex 启动和本地 Multica 工作区开关验收以 `acceptance/codex-multica-workspace-integration.md` 为准；默认路径必须零 CLI 下载/安装、零 profile 创建、零登录、零 daemon 启停/监管、零 Runtime 注册、零 `codex.exe app-server` 启动，并且 Manager 主导航没有独立 `Multica Runtime` 页面。

## 验收项

1. **规格、范围与兼容边界**
   - 通过标准：本 spec 和本 acceptance 均存在；托管运行时是默认关闭的 legacy 高级兼容能力，只在用户显式启用后负责 CLI/daemon 安装、profile、连接和监管；旧的手工 sidecar 连接仍可用且默认关闭；没有 Multica server/数据库内嵌或任务写入入口。
   - 证据：文件存在检查、`git diff --check`、修改文件列表；代码审查确认托管路径与手工路径分离，未调用任务创建/领取/取消/Review API。

2. **固定版本与平台选择**
   - 通过标准：构建产物声明单一固定版本、目标三元组、精确资产名和 SHA-256；当前平台只选择对应 allowlist 条目，不请求 latest 或相近架构。
   - 证据：清单单元测试覆盖 Windows x64、macOS x64/arm64 和不支持平台；构建输出显示版本/平台/asset name/digest；未知版本、未知资产和架构不匹配均拒绝。

3. **资源优先与官方 Release 下载**
   - 通过标准：优先使用发布包内经过同样校验的资源；资源缺失或损坏时只访问固定版本官方 GitHub Release 和允许的官方资产 CDN；任意 URL、latest、非 HTTPS、非 allowlist 重定向被拒绝。最多尝试 3 次，且只重试网络错误、超时、408/425/429/500/502/503/504；永久 4xx、重定向拒绝、取消和校验失败不重试。
   - 证据：本地资源 fixture、fake Release server、重定向和重试分类测试；请求记录只出现固定 tag/asset/checksum 路径并符合尝试次数；离线且无资源时返回 `download_failed`，不阻塞主界面。

4. **SHA-256 和归档安全校验**
   - 通过标准：按校验清单完整文件名解析 SHA-256；摘要不匹配、清单缺失、HTTP 非 2xx、超大文件、错误归档、绝对路径、`..` 穿越、符号链接逃逸和重复覆盖均失败。
   - 证据：校验/归档 fixture 覆盖成功与每类失败；日志和 DTO 只有摘要是否通过和脱敏错误码，不含 token、响应正文或请求头。

5. **临时文件、原子激活与并发锁**
   - 通过标准：下载、解压和入口检查在应用专属临时目录完成；flush/sync 后才原子激活；并发准备只产生一个安装结果；崩溃/取消不留下半成品当前版本。
   - 证据：安装器测试检查临时目录、版本目录、激活元数据和锁；模拟中断、取消、磁盘/权限错误后当前版本内容和指针保持一致。

6. **失败回滚与历史版本**
   - 通过标准：新版本 `--version` 不匹配、可归因于当前二进制的首启失败、health 失败或异常退出时恢复上一验证版本；空/非法 Server URL、连接停用、应用退出中或 sidecar 固定契约错误不得切换版本指针；无旧版本时标记不可用但 CCP 继续启动；回滚不修改供应商/客户端文件。安装、升级、损坏重装和回滚后必须整体刷新托管 sidecar 的 executable、固定工作目录、固定参数及 auto-start，即使 executable 未变化也能修复旧契约，同时保持名称、Server URL、enabled 和普通连接原值。运行中的切换必须在安装锁内完成“验证归属 → 停止 → 原子切换 → 按原状态重启”，外来或不可核验 PID 必须拒绝切换。
   - 证据：双版本 fixture、启动失败和配置拒绝 fixture、同 executable 旧参数 fixture、运行中安装/回滚停止与重启 fixture、外来 PID 拒绝 fixture、操作前后文件哈希/大小/时间戳及连接快照对照；记录回滚原因、恢复版本和退出码。

7. **显式托管连接和 profile**
   - 通过标准：干净环境正常启动不会创建 `managed-multica` 或 `ccp-managed`。用户显式启用 legacy 能力后才幂等创建记录，初始 `enabled/auto_start/supervise=false`；profile 位于独立目录，用户随后可以分别开启生命周期开关。
   - 证据：干净临时用户目录启动两次，连接/profile 均不存在；显式启用后创建一条且字段一致；检查 `%USERPROFILE%\.multica\profiles\ccp-managed\`（或平台对应路径）和 CCP 独立设置命名空间。

8. **用户值不回填、不归一化**
   - 通过标准：用户修改或清空显示名称、server URL、开关或 profile 配置后，保存、刷新、安装、启动和重启均保留原值；不会补尾斜杠、改大小写、替换成 `127.0.0.1:57321` 或其他代理地址。
   - 证据：保存前后 JSON/设置快照和运行日志对照；针对空名称、空 URL、自定义路径和非默认开关执行重启测试；无自动回填调用或字段覆盖。托管保存请求测试还必须证明空字符串可通过，且 `connectionId`、sidecar、可执行文件等普通连接字段被 IPC 拒绝。

9. **显式准备与可选监管**
   - 通过标准：默认 CCP 启动和本地工作区开启/关闭均不准备、下载、安装、登录或启动 daemon。用户进入 legacy 高级兼容面板并确认准备后才允许安装；只有用户再分别开启 `enabled/auto_start/supervise` 且固定版本/profile 已验证时，后续启动才可恢复 daemon。进程使用独立作业对象/日志/环境/端口；托管 daemon 显式禁用自更新和热重载，并只在自身子进程环境中使用保存且校验通过的 Multica Server URL。
   - 证据：默认启动及工作区开关往返的网络、文件和进程 spy 均为零；显式 legacy 流程时间线和状态事件；managed daemon fixture 显示参数严格为 `--profile ccp-managed daemon start --foreground --no-auto-update --no-auto-reload`，子进程环境与端口隔离且父进程环境未变化。

10. **PID 归属、停止回收和有限重启**
    - 通过标准：记录 PID、版本、路径、profile、连接 ID 和启动时间；停止/重启/退出只作用于归属进程树；崩溃按退避和有限次数重启，耗尽后停止，不按通用进程名误杀。独占生命周期锁内回收陈旧 owner：匹配路径才终止并等待退出，PID 路径不匹配只删记录，路径无法核验则保留记录并拒绝接管。
    - 证据：Windows 作业对象/进程树 fixture 或平台等价测试；陈旧 owner 注入测试分别断言“终止并删除”“不终止仅删除”“不终止且保留”；操作前后 managed、Codex/Claude、CCP 代理和无关同名进程存活状态；重启次数达到上限后状态为 `restart_exhausted`。

11. **登录、未授权与敏感信息**
    - 通过标准：登录/退出只作用于 `ccp-managed` profile；token 不出现在 argv、环境快照、URL、日志、错误、前端 DTO 或 CCP 配置；未登录显示 `unauthorized`/`needs_login`，不伪造健康、不无限重启。
    - 证据：fake CLI/secure-store fixture、argv/环境/日志敏感模式扫描；成功、401/403、清空凭据和取消登录均有脱敏状态输出。

12. **只读健康与快照**
    - 通过标准：只有显式开启监管时，Server/daemon 才使用只读 health/version/status 端点并按约定周期执行有界探测；legacy 面板关闭时不得后台轮询。托管 Runtime/Agent/Task 通过不接受 `connectionId` 的专用只读路径读取固定 `managed-multica`，快照保留 `fetched_at`/`source_connection_id=managed-multica`/`stale`；普通快照 IPC 仍拒绝 `managed-multica`。
    - 证据：周期探测和纯状态机测试；专用托管快照命令测试证明无 `connectionId` 输入、固定来源且只读，普通快照命令拒绝保留 ID；fake server/daemon 覆盖 2xx、401/403、404、超时、无效 JSON、进程存在但无健康响应、分页和未知状态；请求审计确认没有任务/租约写入。

13. **安装状态、UI 和命令边界**
    - 通过标准：Manager 主导航没有独立 `Multica Runtime` 页面；普通设置保留本地工作区开关。legacy 托管 UI 只能从明确标记、默认关闭的高级兼容入口打开，打开前不加载网络或进程状态；打开后可查看安装、健康、登录、PID、诊断和只读快照，并执行显式准备/登录/启停/回滚。托管 UI 不暴露任意 executable、shell、任务写入或供应商操作入口；自动恢复门禁要求用户已开启全部 legacy 开关、Runtime 为 `ready` 且 SHA-256 已验证。
    - 证据：前端路由/组件测试与 Playwright 截图断言主导航无 Runtime 项、普通设置有工作区开关、legacy 面板默认关闭且无后台请求；显式打开后验证状态、按钮、错误态与专用 IPC 边界。

14. **供应商、代理和客户端隔离**
    - 通过标准：安装、登录、健康、刷新、启动、停止、重启和回滚前后，供应商列表/名称、active Profile、上游 URL、`config.toml`、`auth.json`、Claude 配置及 CCP 代理监听状态不变；Multica URL 永不被改为 CCP 本地代理 URL。
    - 证据：操作前后对相关文件执行 SHA-256/大小/时间戳比较；供应商连通性和代理端口探测；日志/代码扫描确认没有 `switch_relay_profile_in_home` 或供应商 URL 写入路径。

15. **故障隔离与恢复**
    - 通过标准：下载、校验、磁盘、权限、认证、网络、CLI 版本、daemon 崩溃和重启耗尽均给出独立错误码；失败不阻塞主界面、供应商、代理或 Codex/Claude；重启后从原子元数据恢复状态。`install-failure.json` 只持久化稳定错误码和时间戳，不含 URL、路径、响应或原始错误；存在有效当前版本时仍报告 `ready`，成功安装清除旧失败记录。
    - 证据：安装失败持久化/脱敏/清除 fixture、故障状态转移和恢复日志；主界面可继续完成供应商/客户端操作；未配置 Multica 模式启动日志和当前版本/上一版本指针一致。

16. **最小精度定向测试阶段**
    - 通过标准：每次实现变更只运行直接相关测试及直接影响面测试，并记录实际输出；不以整仓库测试替代针对性证据。
    - 建议证据命令（按当前变更选择，不要求每次全部执行）：

      ```powershell
      cargo test -p claude-codex-pro-core managed_runtime -- --nocapture
      cargo test -p claude-codex-pro-core multica -- --nocapture
      cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem multica -- --nocapture
      npm --prefix apps/claude-codex-pro-manager run check
      git diff --check
      ```

      - 资产/下载变更至少覆盖第 2-5 项；监管变更至少覆盖第 9-10 项；profile/URL 变更至少覆盖第 7-8、14 项；UI/命令变更至少覆盖第 13 项及对应 Rust 命令测试。

17. **最后一轮大范围回归验收**
    - 通过标准：功能冻结、定向测试通过后，完整 workspace、前端、Release 和现场 smoke 全部通过；默认 `target/release` 保留本次构建产物；供应商/代理/Codex/Claude 回归证据齐全。
    - 必需证据命令：

      ```powershell
      cargo fmt --check
      cargo test --workspace
      npm --prefix apps/claude-codex-pro-manager run check
      npm --prefix apps/claude-codex-pro-manager run vite:build
      cargo build --release
      Get-Item target/release/claude-codex-pro.exe | Select-Object FullName,Length,LastWriteTime
      git diff --check
      ```

      - 现场 smoke 先覆盖干净用户目录默认启动零下载/零 profile/零 daemon；再由用户显式开启 legacy 流程，覆盖固定版本和 SHA-256、profile/连接、登录未授权、daemon 启停/崩溃回收、供应商 URL/代理端口保持不变。

## 必需证据汇总

- 固定版本/平台清单、资源优先级、官方 Release 请求和 SHA-256 校验输出。
- 临时安装、原子激活、并发锁、首启检查、失败回滚和历史版本指针证据。
- 默认启动零 `managed-multica`/`ccp-managed`/下载/登录/daemon，以及显式启用后用户值不回填/不归一化证据。
- 显式启用监管后的 PID 归属、端口隔离、进程树回收、有限重启和不误杀证据。
- 登录/未授权和日志、argv、环境、DTO 脱敏扫描结果。
- 只读健康/快照、过期状态、错误分类和故障隔离结果。
- 主导航无独立 Runtime 页面、普通设置保留工作区开关、legacy 高级兼容面板默认关闭的组件/截图证据。
- 操作前后供应商、Profile、`config.toml`、`auth.json`、Claude 配置及代理状态哈希/大小/时间戳对照。
- 最小精度阶段的实际定向测试输出和最后一轮完整回归、Release 产物元数据、现场 smoke 记录。

## 失败条件

- 下载 latest、任意外部 URL、非 allowlist 主机/资产、跳过 SHA-256 或执行未验证归档/脚本。
- 激活半成品、安装失败覆盖唯一可用版本、回滚修改非托管文件或并发安装造成指针/目录不一致。
- 默认或重启流程覆盖用户 server URL/名称/profile/开关，或把任一 URL 改为 `http://127.0.0.1:57321/v1` 等 CCP 代理地址。
- 自动监管按通用进程名误杀、复用 CCP helper/watchdog/固定端口、无限下载/重启或把进程存在误报为健康。
- token、Cookie、Authorization、原始响应、完整 argv/环境或完整本机路径泄露。
- Multica 故障阻塞 CCP、供应商、代理、Codex/Claude，或过期快照显示为实时健康。
- 默认启动或本地工作区开关触发 CLI 下载/安装、profile 创建、登录、daemon、Runtime 注册或 `codex.exe app-server`。
- Manager 主导航仍显示独立 `Multica Runtime` 页面，或删除该页时一并删除普通设置中的本地工作区开关。
- 未按“定向测试 → 最后一轮大回归”的阶段策略执行，或把未执行的验证写成通过。

## 非范围检查

- 不验收 Multica server、PostgreSQL、Redis、Web/Electron/移动端的安装、迁移、性能、商业托管或真实任务执行。
- 不验收 Agent 调度、Review、盘古记忆写入、Skills 同步或新的供应商功能。
- 不要求单一可执行文件内嵌完整 Multica 平台；只要求发布包资源优先和官方 Release 缺失资源自动下载。

## 已知风险

- 官方 Release 资产命名、校验清单格式和 CLI 命令可能随固定版本变化；每次升级必须同步清单、能力映射和 fixture。
- Windows 作业对象、文件替换、杀毒软件锁定和权限语义需要目标系统现场验证。
- 首次下载需要网络和 GitHub 可达性；离线用户只能使用已打包或已验证的本地版本，UI 必须说明限制。
- server URL、远端认证和健康端点属于外部服务契约；不兼容时只能显示未授权/不可用，不能通过修改 CCP 供应商配置规避。
