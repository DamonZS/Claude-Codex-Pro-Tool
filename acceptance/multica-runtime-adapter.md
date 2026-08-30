# 验收标准：Multica Runtime Adapter 首个增量

验证对象：`spec/multica-runtime-adapter.md`

## 验收项

1. **文档与范围**
   - 通过标准：spec 和本 acceptance 均存在；实现只覆盖外部连接、只读健康检查、只读状态快照和可选 sidecar 监管。
   - 证据：文件存在检查、`git diff --check`、修改文件列表；代码审查确认没有任务写入接口。

2. **连接配置与 URL 原值**
   - 通过标准：可新增、编辑、启用/停用和删除连接；`server_url` 的协议、主机、路径和 API 前缀不被改写；重复连接不会生成供应商记录或覆盖显示名称。
   - 证据：连接配置定向测试；保存前后配置快照对照；结果中不得出现 `127.0.0.1:57321`、`57331` 等 CCP 代理地址替换。

3. **凭据脱敏**
   - 通过标准：令牌默认隐藏并以安全引用保存；清空令牌不会自动回填；Tauri DTO、日志、错误、数据库、argv 和环境中均无令牌原文。
   - 证据：脱敏测试和敏感模式扫描；只报告“已配置/未配置”，不输出密钥值。

4. **健康检查分类**
   - 通过标准：Server 和 daemon 只调用只读端点；能区分 `unconfigured`、`checking`、`healthy`、`degraded`、`unreachable`、`unauthorized`、`invalid_response`、`stopped`；进程存在或 HTTP 请求发出不能单独判定健康。
   - 证据：fake server/daemon fixture 覆盖 2xx、401/403、404、超时、TLS/DNS、无效 JSON、进程存在无健康响应；测试输出含状态、端点类别、耗时和脱敏错误码。

5. **超时、取消和并发**
   - 通过标准：连接和总超时有界；重复刷新会去重或取消旧请求；并发受上限约束；失败后 loading 恢复。
   - 证据：异步测试模拟阻塞、重复刷新和取消，断言在约定时间返回且前端可再次操作。

6. **sidecar 显式监管**
   - 通过标准：默认不启动；用户确认后只执行已验证本地可执行文件；记录 PID/连接归属；不复用 GUI helper/watchdog 或端口 `57321`、`57331`、`57320`、`9230`。
   - 证据：sidecar fixture 覆盖默认关闭、启用、无效路径和启动失败；命令行、环境、监听端口只输出非敏感摘要；源码检查无 Codex/Claude GUI 启动调用。

7. **sidecar 回收与不误杀**
   - 通过标准：停止/重启只作用于已验证 PID 及进程树；崩溃/重启耗尽状态明确；不按通用进程名杀进程，不影响 CCP 代理、Codex/Claude GUI 或无关 Electron 进程。
   - 证据：进程树 fixture 或 Windows 作业对象测试；操作前后记录目标与非目标 PID 存活状态。

8. **只读状态快照**
   - 通过标准：Runtime、Agent、Task 支持分页或数量上限；快照有 `fetched_at`、来源连接和 `stale`；刷新失败保留旧快照并标记过期；未知字段/状态不丢弃整批数据。
   - 证据：fixture 覆盖分页、未知字段、坏记录、空结果；解析测试通过；返回 DTO 无原始消息、完整路径、请求头或凭据。

9. **管理工具只读 UI**
   - 通过标准：提供连接编辑、测试连接、刷新和 sidecar 启停；Runtime/Agent/Task 仅查看脱敏详情，不出现任务创建、取消、重试、编辑、删除入口；加载、空态、错误、过期和禁用状态可见。
   - 证据：前端组件测试或 Playwright 截图；键盘/无障碍检查确认状态文字、按钮名称、忙碌态和确认文案。

10. **CCP 全局配置和代理不变**
    - 通过标准：连接操作、健康检查、快照刷新和 sidecar 生命周期前后，供应商列表、active Profile、`config.toml`、`auth.json`、Claude 配置及代理监听状态不变；Multica 不可达不阻塞 CCP。
    - 证据：操作前后对相关文件执行 SHA-256/大小/时间戳只读比较；代理端口健康探测；供应商/客户端回归测试；git diff 无无关配置写入。

11. **诊断脱敏与故障隔离**
    - 通过标准：错误可区分认证、网络、超时、无效响应、sidecar 退出和数据过期；不含 API Key、Bearer Token、Cookie、Authorization、原始响应正文；适配器故障不影响主界面和 GUI 启动。
    - 证据：错误分类/日志测试；敏感模式扫描；抽样日志仅含动作、连接 ID、端点类别、状态、耗时和错误码。

12. **构建与回滚**
    - 通过标准：Rust、Tauri、前端检查及 Release 构建通过；删除/禁用适配器后 CCP 能以未配置 Multica 模式启动；默认 Release 产物为本次构建。
    - 证据：必需命令全部退出码 0；记录 `target/release/claude-codex-pro.exe` 路径、大小、时间；回滚启动日志证明代理和 GUI 可用。

## 必需验证命令

```powershell
Test-Path spec/multica-runtime-adapter.md
Test-Path acceptance/multica-runtime-adapter.md
git diff --check
cargo fmt --check
cargo test -p claude-codex-pro-core multica -- --nocapture
cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem multica -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo build --release
Get-Item target/release/claude-codex-pro.exe | Select-Object FullName,Length,LastWriteTime
```

若实现采用不同测试过滤器，交付报告必须列出实际测试名称和输出。Release 构建前只结束路径明确位于本项目构建目录中的旧 CCP 进程。

## 必需证据汇总

- 连接 CRUD、URL 原值、令牌脱敏和健康分类测试输出。
- 超时/取消/并发、sidecar PID 归属、端口隔离、进程树回收和不误杀证据。
- Runtime/Agent/Task 分页、过期快照、未知状态兼容和只读 DTO 证据。
- 操作前后供应商、Profile、`config.toml`、`auth.json`、Claude 配置与代理状态的哈希/状态对照。
- 前端检查、Vite 构建、Rust 测试、Release 构建及产物元数据。

## 失败条件

- 健康或刷新改变 Multica Task/租约，或出现任务创建、取消、重试等写入入口。
- 调用 `switch_relay_profile_in_home`，或修改 active Profile、供应商名称、上游 URL、`config.toml`、`auth.json`、Claude 配置。
- 出现 `http://127.0.0.1:57321/v1` 等 CCP 代理地址覆盖 Multica/供应商 URL。
- sidecar 与 GUI 启动器、helper、watchdog 共用固定端口或按名称误杀进程。
- 任何日志、前端、数据库、argv 或环境泄露密钥、Cookie 或完整请求正文。
- Multica 不可达时阻塞 CCP 或把过期快照显示为实时健康。

## 非范围检查

- 不验收 Multica server、PostgreSQL、Redis 的安装、迁移、性能或商业托管。
- 不验收 Multica Web/Electron/移动端 UI 改造、真实任务执行、Agent 调度、Review、盘古记忆写入或 Skills 同步。
- 不要求把 Multica 打包成 CCP 内嵌服务或单一可执行文件；现有供应商路由、协议代理、注入和 GUI 启动只检查未被回归。

## 已知风险

- 外部 API 端点和返回结构可能变化，需依赖能力探测与向前兼容解析。
- Windows 作业对象、权限和进程树语义需在目标 Windows 版本现场验证。
- 用户配置的 HTTP/局域网服务存在中间人风险，默认 HTTPS 和最小令牌权限不能替代服务端 TLS/访问控制。
- 连接状态是外部快照，不保证任务最终业务状态，UI 必须显示来源和新鲜度。
