# cc-switch → CCP 可移植能力提案

> 状态：**提案，未实现**。本文只回答「cc-switch 有什么值得拿进 CCP」，不包含代码改动。
> 分析对象：`D:\Project\cc-switch`（`cc-switch` v4.0.26，MIT，Tauri 2 + React 18 + TypeScript）。
> 分析方式：模块体量与模块级文档注释 + 公开 API 签名 + 与 CCP 现有模块逐一对照；未逐行读实现。

## 结论摘要

两者是同一类产品：本机多 Agent 配置控制台。cc-switch 覆盖面更宽（`src-tauri/src` 下有 13 个 `*_config.rs`：`codex_config.rs` 261.4 KB、`claude_desktop_config.rs` 116.5、`dsh_config.rs` 109.5、`hermes_config.rs` 95.4、`opencode_config.rs` 58.2、`omp_config.rs` 51.6、`app_config.rs` 47.7（应用注册表，`AppType` 定义在 `:466`）、`kimi_config.rs` 45.3、`agy_config.rs` 37.7、`openclaw_config.rs` 35.0、`mcode_config.rs` 26.1、`gemini_config.rs` 21.3、`grok_config.rs` 15.9；另有完整代理托管与托盘面板），CCP 在**数据侧**更强（用量索引、会话血缘合并、蒸馏、Provider Sync）。

真正值得拿进 CCP 的是 **4 项能力**，按「CCP 完全没有 + 直接收益」排序：

| 优先级 | 能力 | cc-switch 落点 | CCP 现状 | 价值 |
|---|---|---|---|---|
| **P0-1** | 订阅 / 编程套餐额度查询 | `services/subscription.rs` 87.5 KB、`services/coding_plan.rs` 135.7 KB | **0 覆盖** | 用量看板从「花了多少」补成「还剩多少」 |
| **P0-2** | 安全写入引擎（hash 校验 + 字节级备份 + 崩溃恢复） | `live/engine.rs` 10.7 KB、`mode/operation.rs` 44.3 KB | 直接 `atomic_write` 写文件 | 消除「改配置改坏 / 切供应商后状态错乱」这一类故障 |
| **P1-1** | Skills SSOT + 按应用同步 | `services/skill.rs` 325.0 KB | `unified_tool_inventory.rs` 78.0 KB（清单为主） | 一份 Skill 装一次，多应用生效 |
| **P1-2** | 供应商深链分享 / 导入 | `deeplink/provider.rs` 68.3 KB、`deplink.html` 160.6 KB | 只能从 cc-switch 单向导入 | 一条链接把供应商发给别人 |
| **P2** | 托盘面板、WebDAV 配置同步、多安装方式卸载、图像能力注册表 | `tray.rs` 189.6 KB、`commands/webdav_sync.rs` 15.5 KB、`commands/tool_uninstall.rs` 70.2 KB、`model_capabilities.rs` 12.6 KB | 无 / `ccp_backup.rs` 45.2 KB（本地备份） | 体验与运维补强 |

## 一、逐域对照

### 1. 供应商配置

cc-switch 的强项是**写入正确性**，不是配置项数量。`src-tauri/src/live/engine.rs:1-8` 的模块注释写明了它的唯一写文件路径：

> 读字节并记下 hash → 按格式解析，解析失败就停 → 在内存里改关键字段 → 序列化成新字节（`plan`）→ 写好临时文件（`stage`）→ 发布前对这个文件做一次字节级备份（每个文件只做一次）→ 最后重读一次比对 hash，没变才 rename，变了就以新内容为底重算。

API 面：`DeviceStore:30`、`LiveFile:59`、`digest():81`、`sha256_hex():86`、`read_current():94`、`Planned:107`、`plan(&LiveFile, &dyn LivePatch):128`、`stage():150`、`ensure_first_write_backup():164`、`AppWriteGuard:189`。多文件写前意图与崩溃恢复在 `mode/operation.rs`。

配套的是 `src-tauri/src/mode/controller.rs:1-12` 的双模式控制器：每个可代理应用只有一个持久模式（`live-state.json`，设备本地），所有操作都是「DB + 模式状态 → 客户端文件」的**投影**，可重复执行、不依赖快照。API：`enter:720`、`enter_with_route:727`、`exit:901`、`exit_apps_in_mode:916`、`exit_all:1014`、`detach_all:1046`、`RouteSwitchResult:1070`、`switch_route_locked:1084`、`switch_route:1154`、`switch_route_for_failover:1164`。

CCP 现状：`relay_config.rs` 133.3 KB 负责 Codex 中转配置、`agent_providers.rs` 37.7 KB + `agent_providers/` 8 个写入器（gemini / grok / hermes / mcode / openclaw / opencode / pi / workbuddy）分别改各应用文件。**没有 hash 校验、没有每文件字节级备份、没有写前意图/崩溃恢复。**

→ 建议：只移植 `live/engine.rs` 的写入契约（plan → stage → 首次备份 → 重读比对 → rename）与 `operation` 的崩溃恢复，不搬 `controller.rs` 的整套模式机（CCP 的路由/中转模型不同）。

### 2. 订阅与套餐额度（CCP 完全没有）

- `services/subscription.rs:1-6`：「官方订阅额度查询服务。读取 CLI 工具的已有 OAuth 凭据，查询官方订阅额度。第一层：仅读取凭据，不实现登录/刷新。」类型：`CredentialStatus:18`、`QuotaTier:30`、`ExtraUsage:48`、`ResetCredits:59`、`SubscriptionQuota:68`；入口 `read_claude_default_plan():209`、`get_claude_account_quota():1670`、`ClaudeAccountQuota:1768`。
- `services/coding_plan.rs:1-6`：「Token Plan / 编程套餐额度查询服务。支持 Kimi For Coding、智谱 GLM、MiniMax、ZenMux、火山方舟、OpenCode Go、Command Code 的套餐额度查询。复用 subscription 的 `SubscriptionQuota` / `QuotaTier` 类型。」火山方舟的 `base_url` 形如 `https://ark.cn-beijing.volces.com/api/plan[/v3]`（Agent Plan）或 `/api/coding[/v3]`（Coding Plan）。

CCP 现状：`crates/claude-codex-pro-core/src` 全目录 grep `quota` / `SubscriptionQuota` = **0 命中**。用量看板（`local_usage.rs` / `usage_index.rs`）只有消耗，没有余量。

→ 建议：P0。移植「读凭据 → 查额度 → 统一 `QuotaTier` 结构」，在看板顶部按供应商显示余量百分比与重置时间。凭据只读、不上报，符合 CCP 现有安全边界。

### 3. MCP

cc-switch 按应用拆文件：`mcp/codex.rs` 42.2 KB、`hermes.rs` 20.9、`pi.rs` 19.1、`opencode.rs` 12.5、`agy.rs` 10.0、`omp.rs` 8.8、`mcode.rs` 8.6、`grokbuild.rs` 8.6；根目录另有 `claude_mcp.rs` 17.8 KB。

CCP 现状：`unified_tool_inventory.rs` 78.0 KB 做「Claude 与 Codex 的插件/Skills/MCP 合成一张清单」，偏只读盘点与应用开关，写入路径少。

→ 建议：P1。搬「按应用分文件的 MCP 写入器」这个结构（每应用一个文件、同一套 `read/apply/remove`），CCP 已经在 `agent_providers/` 用了同样的组织方式，接得上。

### 4. Skills

`services/skill.rs:1-6`：

> Skills 服务层。v3.10.0+ 统一管理架构：SSOT（单一事实源）`~/.cc-switch/skills/`；安装时下载到 SSOT，按需同步到各应用目录；数据库存储安装记录和启用状态。

CCP 现状：`unified_tool_inventory.rs` 盘点 + 各应用目录各自为政，没有 SSOT。

→ 建议：P1。只搬「SSOT 目录 + 同步到各应用 + 记录启用状态」三件事，不搬 325 KB 的完整实现（含远端下载）。

### 5. 提示词

cc-switch：`prompt_files.rs`（`prompt_file_path(app: &AppType) -> Result<PathBuf, AppError>`）、`prompt.rs` 0.5 KB；前端 `src/` 有对应编辑界面。

CCP 现状：`prompt_library.rs` 61.3 KB 已有提示词库。

→ 建议：不搬。只参考它的「按应用定位提示词文件路径」这一点，若 CCP 还没按应用分路径可补。

### 6. 会话

cc-switch：`session_manager/`（`content.rs` 53.1 KB、`cache.rs` 32.3、`model.rs` 22.3、`hermes_wsl.rs` 6.6）、按应用的会话用量 `services/session_usage_codex.rs` 253.4、`session_usage_pi.rs` 112.9、`session_usage_grokbuild.rs` 66.5、`session_usage.rs` 103.3。

CCP 现状：`claude_sessions.rs` 84.6 KB、`request_history.rs`（data 包）、`local_usage.rs` 已有会话名/血缘合并/蒸馏，功能更深。

→ 建议：不搬。价值在「按应用的用量口径」，可作为 CCP 覆盖面检查清单。

### 7. 应用与设置

- `commands/tool_uninstall.rs:1-2`：「Uninstall the detected installation, retaining configuration and user data. Never fall back to a different package manager or delete an arbitrary directory.」每种检测到的安装各有自己的 plan，用户选一个，其余保留。
- `commands/webdav_sync.rs` 15.5 KB：配置同步到 WebDAV。
- `tray.rs` 189.6 KB + `tray_panel.rs` 50.0 KB + `src/tray-panel/`：托盘面板。
- `model_capabilities.rs`：图像输入能力 `Unknown` 与 `Supported` 明确区分；Codex catalog 对未知模型 fail-open 视为可图像，代理整流器对未知模型不改请求体。

CCP 现状：`ccp_backup.rs` 45.2 KB（本地备份）、Tauri 托盘（`TrayIcon` 有命中）、`model_catalog.rs` 60.2 KB。

→ 建议：P2。`tool_uninstall` 的「多安装方式各一个 plan、只卸安装留数据」最值得抄；`model_capabilities` 的 fail-open 口径可补进 `model_catalog.rs`。

## 二、不建议搬的部分

| 模块 | 体量 | 理由 |
|---|---|---|
| `proxy/forwarder.rs` + `proxy/handlers.rs` 等 | 358.7 + 224.7 KB 起 | CCP 已有 `protocol_proxy.rs` 369.8 KB，两套转发层并存只会撕开两套 bug 面 |
| `commands/misc.rs` | 503.7 KB | 杂项大杂烩，无清晰边界，搬过来只会污染命令层 |
| `database/schema.rs` + `backup.rs` | 165.7 + 139.6 KB | 与 CCP 的 `ccp.db` / `usage-index-v1.db` 双库结构冲突 |
| `mode/controller.rs` 整套模式机 | 350.3 KB | CCP 的路由模型是「中转 profile + 本地代理」，语义不同，只取写入契约即可 |
| `src/App.tsx` 66.6 KB、`components/` 全家桶 | — | 前端是 Radix + i18next 另一套体系，CCP 用自写组件 + zh/en 双语文案，混用会双倍维护 |

## 三、落地顺序与验证方式

每项都按仓库规则先写 `spec/` 与 `acceptance/`，再实现：

1. **P0-1 额度查询** → spec 定义「凭据只读、失败降级不阻塞看板」；验收：给定固定响应 fixture，额度百分比与重置时间解析正确，无凭据时不显示该卡片、不报错。
2. **P0-2 写入引擎** → spec 定义 plan/stage/rename 契约与「外部进程改了文件就重算」；验收：并发写同一文件不丢改动、写入中断后能恢复到备份、备份每文件只产生一次。
3. **P1-1 Skills SSOT** → 验收：同一条 Skill 安装后各应用目录内容一致，卸载只删记录不删用户文件。
4. **P1-2 深链** → 验收：一条带供应商参数的链接能导入且不覆盖现有 profile；非法链接给出可读错误。
5. **P2** 按需。

## 四、附：一项已完成项

「重启 Codex」左侧的 GitHub Star 按钮已实现并交付：`apps/claude-codex-pro-manager/src/components/AppShell.tsx:512-515`（`onOpenGithub` → `Star` 图标 + 「点个 Star」），`App.tsx:3060` 接 `openExternalUrl("https://github.com/DamonZS/Claude-Codex-Pro-Tool")`。规格与验收见 `spec/github-star-entry.md`、`acceptance/github-star-entry.md`。