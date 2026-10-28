# codex-bot 注入面板

## 背景

用户希望把本机 DSH 里的 Bot 团队直接呈现在 Codex 界面里（原话：我在 codex 界面能看到 bot 界面吗），并且明确不走 MCP 兜底，而是采用注入方式（原话：制作注入面板，mcp兜底毫无意义，将项目所有的ds-bot都改成codex-bot）。

CCP 已有成熟且在生产使用的 Codex 注入通道：`assets/inject/renderer-inject.js` 由启动器经 CDP `Page.addScriptToEvaluateOnNewDocument` 注入到 Codex 渲染进程，本地 helper 通道监听 `http://127.0.0.1:57321`，浏览器来源请求需带 `x-claude-codex-pro-token`。因此把 Bot 团队搬进 Codex 界面不需要改动 Codex 安装文件，也不需要新增端口。

数据源是 DS-Bot 插件落在 `$DSH_HOME/bot` 下的只读快照（`state.json`、`chat-index.sqlite`、`usage.sqlite`）。本功能在 CCP 内统一命名为 **codex-bot**：凡本项目涉及该能力的代码、路由、文件、文案与文档一律使用 codex-bot 字样。

## 目标

包含：

- Codex 左栏注入入口「codex-bot」，以及只读面板：Bot 列表、房间列表、最近消息、用量摘要。
- Rust 侧只读数据层与 helper 路由（`/codex-bot/*`）。
- 空态、错误态与数据源未就绪时的降级；面板可收起并记住状态。
- 单测（Rust 夹具 + 注入脚本 `node --test`）与真机验证。

不包含：

- 从面板发消息、改 Bot、改配置（v1 只读）。
- MCP 包装或 MCP Apps 方案。
- DS-Bot 插件源码、DSH 本体或 Codex 官方文件的改动。
- 跨机器访问、远程端口暴露、为其他客户端提供接口。

## 用户视角

1. 用户在 CCP 里正常启动 Codex（启动器已带调试端口与注入通道）。
2. Codex 左栏出现「codex-bot」入口，点击展开面板。
3. 面板显示 Main Bot 与其团队：每个 Bot 的名称、当前会话、最近活动；房间（群聊）列表含成员数与最后一条消息预览；用量摘要（今日 / 7 天 token，按 Bot 排行）。
4. 数据源未就绪（DSH 未运行、`DSH_HOME` 未配置、数据库缺失）时，面板显示引导文案与重试按钮，Codex 其余功能不受影响。
5. 面板可收起，开合状态在下次打开时保持；明暗主题下都可读。

## 功能要求

### F1 数据源与解析

- `codex_bot_home()`：`DSH_HOME`（空白视为未设置）→ 默认 `~/.dsh`，再拼 `bot`；与 DS-Bot 的 `dshHomeOf` 解析规则一致。
- `state.json`：解析 `revision`、`mainBotIds`、`bots{<bot 会话 id> -> {id,name,role,color,avatar,capabilities,createdAt}}`、`rooms{id -> {id,name,members[]}}`。实测 `bots` 的键就是 Bot 的会话 id（`session-<uuid>`），且 Bot 对象上**没有 `workDir` 字段**；字段缺失一律按不存在处理，不做非空假设。文件缺失或 JSON 解析失败 → 空快照 + `reason`。
- `chat-index.sqlite`（只读）：表 `lines(text,session,seq,time,who)`；按 `session` 过滤、按 `seq` 倒序取最近 N 条。实测该库**可以与 `usage.sqlite` 不同时存在**（本机缺失 `chat-index.sqlite`），因此「就绪」按分区判定：`summary.available` 反映数据根与 `state.json`，消息区独立降级为空并给出 `reason`。
- `usage.sqlite`（只读）：表 `calls(session,time,provider,model,kind,live,input,cache_read,cache_write,output)`、`owners(session,owner,name)`、`kinds(session,kind)`（实测另有 `marks(session,through,fold,sealed)`，不读即可）；按天聚合 token，并按 `owners` 归属到 Bot。
- 所有 SQLite 一律只读打开（`OpenFlags::SQLITE_OPEN_READ_ONLY`），复用 workspace 已有的 `rusqlite 0.32`（bundled）。实测 `usage.sqlite` 处于 WAL 模式（同目录有 `-wal` / `-shm`）：只读打开不得依赖创建 `-shm`；打不开或忙时按下一行降级。
- 缺文件、缺表、`SQLITE_BUSY` → 返回上一次有效快照（进程内短 TTL 缓存，默认 15 s；`state.json` 以 mtime + size 判新旧），并带 `stale: true`；绝不 panic。

### F2 helper 路由

挂在既有 `handle_helper_connection` 派发中，鉴权规则不变（浏览器来源必须带 token；无 Origin 的原生客户端放行）。

- `GET /codex-bot/summary` → `{available, reason?, revision, botCount, roomCount, mainBotId, stale, chatIndexAvailable, usageAvailable, usage:{today:{input,output,total}, week:{input,output,total}}}`（整体 `available` 只反映数据根与 `state.json`；两个 `*Available` 表示各分区是否可用）
- `GET /codex-bot/bots?limit=` → `[{id, name, role?, color?, currentSessionId?, lastActivityAt?, messageCount?, usage:{input,output,total}}]`；默认 20，上限 100。
- `GET /codex-bot/rooms?limit=` → `[{id, name, memberCount, lastMessageAt?, preview?}]`。
- `GET /codex-bot/messages?session=<id>&limit=` → 正常时 `[{seq, time, who, text}]`（`text` 截断至 2000 字符，按 `seq` 倒序）；`chat-index.sqlite` 缺失时返回 `{available:false, reason:"missing-database"}`（裸数组无法表达“不可用”），HTTP 仍为 200。
- 数据源未就绪统一返回 `200` + `{available:false, reason}`，不使用 5xx；参数非法返回 `400`。

### F3 注入面板

- 新文件 `assets/inject/codex-bot-panel.js`，自执行脚本；由 `crates/claude-codex-pro-core/src/assets.rs` 以独立常量 `include_str!` 引入，并在 `injection_script_with_settings()` 中追加到渲染脚本之后，与外层共用同一条注入。
- 契约：读取 `window.__CODEX_SESSION_DELETE_HELPER__`（helper 基址）与 `window.__CLAUDE_CODEX_PRO_HELPER_TOKEN__`（token，请求头 `x-claude-codex-pro-token`）。
- 取数通道：优先走既有 CDP 桥 `window.__codexSessionDeleteBridge(path, payload)` —— 实测 Codex 页面（`app://-/index.html`）的 CSP 不允许直连 `http://127.0.0.1:<port>`，`fetch`/`no-cors` 一律 `Failed to fetch`；桥返回体与 helper 的 HTTP body 完全一致（`lib.rs` 同款 `handle_route` 结果），失败返回 `{status:"failed", message}`。桥不存在时（手工注入、单测）回退直连 helper 并带 token。
- 桥侧路由：`crates/claude-codex-pro-core/src/routes.rs` 的 `handle_bridge_request` 命中 `codex_bot::is_codex_bot_path(path)` 时转到 `codex_bot::bridge_payload(path)`（含查询串，故 `?session=`/`?limit=` 语义与 helper 一致）。
- 挂载锚点按既有选择器体系回退：`nav[role="navigation"] button.sidebar-item`、`aside.app-shell-left-panel button.sidebar-item`、`nav[role="navigation"] button.h-token-nav-row.w-full`；锚点缺失时不报错，最多重试若干次后放弃。
- 面板结构：标题行（codex-bot、刷新、收起）、Bot 列表、房间列表、用量区、选中会话的最近消息；所有用户数据用 `createElement` / `textContent` 渲染。
- 开合状态存 `localStorage["codex-bot.panel.collapsed"]`。

### F4 降级与无侵入

- helper 不可达、解析失败或数据源未就绪 → 面板显示「数据源未就绪（需运行带 DS-Bot 的 DSH）」与重试按钮；控制台不打印 token、也不打印消息全文。
- 实测本机常态是：1 个 Bot、0 个房间、`chat-index.sqlite` 缺失、`usage.sqlite` 为空表（0 行）。面板必须把这四种情况都渲染成正常界面（单 Bot、房间空态、消息空态、用量 0），不得出现空白、NaN 或报错。
- 面板所有 DOM 操作包在 try/catch；失败只影响面板自身，不影响 Codex 原生 UI。

## UI / 交互要求

- 入口：左栏一行，图标 + 文案「codex-bot」，与既有侧栏项同层级，沿用其 hover / active 样式。
- 面板宽 320–420 px，可收起为入口行；滚动区独立；长文本省略号并带 `title`。
- Bot 列表项：名称为主，角色 / 当前会话为副；房间项：名称 + 成员数 + 最后一条预览（`who：text`，截断 60 字）。
- 用量区：今日 / 7 天合计（千位分隔，0 也要显示）+ 前 5 个 Bot 排行（条形）。
- 状态：骨架屏（首次加载）、空态（无 Bot / 无房间 / 无消息）、错误态（未就绪 / 失败）。
- 主题：读取 Codex 主题变量（与既有注入脚本相同的取值方式），明暗主题都可读；不覆盖 Codex 原生控件。

## 数据与接口要求

- 输入：`DSH_HOME`（环境变量可选）、`state.json`、`chat-index.sqlite`、`usage.sqlite`。
- 输出：F2 约定的 JSON 结构。
- `reason` 取值：`missing-home`、`missing-state`、`missing-database`、`parse-error`、`busy`、`unsupported-schema`。
- 错误处理：一律降级为 `available:false` + `reason`（分区缺失时用 `messagesAvailable:false` + 该区 `reason`）；日志仅在 debug 下打印路径，不打印消息内容。
- 鉴权：沿用既有 helper 规则，不新增鉴权方式。

## 技术约束

- 复用既有注入与 helper 通道，不新增监听端口。
- Rust 侧只读：不写 DS-Bot 数据目录，不写 Codex 目录、不写 `%USERPROFILE%\.dsh`。
- 不新增第三方依赖（`rusqlite` 已在 workspace 依赖中）。
- 命名统一为 codex-bot（命令、路由、文件、文案、文档）；外部 DS-Bot 项目及其数据目录名保持原名不动。
- 面板脚本随二进制打包（`include_str!`），`renderer_fingerprint()` 会随脚本内容变化。
- 不打包 Tailwind：视觉只取 ReUI / AI Elements 的结构、状态与 tokens，用纯 DOM + CSS 变量实现。

## 交付范围

- `spec/codex-bot-injection-panel.md` 与本文件对应的 `acceptance/codex-bot-injection-panel.md`。
- Rust：core 新增 codex-bot 数据模块（解析 + 缓存 + 四个查询）与 helper 路由，附单测（夹具在测试内用临时目录生成，不提交二进制夹具）。
- 注入前端：`assets/inject/codex-bot-panel.js` 与 `assets/inject/codex-bot-panel.test.mjs`（`node --test`），以及 `assets.rs` 接线（由 lead 负责）。
- 验证：`cargo fmt`、`cargo test --workspace`、注入脚本单测、`vite:build`、`cargo build --release`，以及真机（CCP 启动 Codex）验证证据。

## 变更记录

- 2026-10-28：初版（只读面板、不走 MCP、统一 codex-bot 命名）。
- 2026-10-28（四）：真机验证发现 Codex 页面 CSP 阻断 `fetch` 直连 helper（`Failed to fetch`，`no-cors` 同样失败），取数通道改为 CDP 桥优先、直连回退；桥侧新增 codex-bot 路由转发。
- 2026-10-28（三）：按实现落定分区字段名 —— `/summary` 用 `chatIndexAvailable` / `usageAvailable` 表示分区可用性（整体 `available` 不变）；`/messages` 在 chat index 缺失时返回对象而非裸数组。
- 2026-10-28（二）：按本机真实数据补数据契约 —— `bots` 以 Bot 会话 id 为键且 Bot 对象无 `workDir`；`chat-index.sqlite` 可以与 `usage.sqlite` 不同时存在（消息分区独立降级，`summary` 增 `messagesAvailable`）；`usage.sqlite` 为 WAL 模式（同目录有 `-wal` / `-shm`），只读打开不得依赖创建 `-shm`；常态为 1 Bot / 0 房间 / 0 用量，面板四类空态都必须正常渲染。