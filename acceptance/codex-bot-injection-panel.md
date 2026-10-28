# codex-bot 注入面板 · 验收标准

验证对象：[spec/codex-bot-injection-panel.md](spec/codex-bot-injection-panel.md)

## 通过标准

1. **数据层可用**：给定夹具（`state.json` + `chat-index.sqlite` + `usage.sqlite`），数据层返回 `available:true`，且 `revision`、`botCount`、`roomCount`、`mainBotId` 与夹具一致；缺 `state.json` 时返回 `available:false`、`reason:"missing-state"`。
2. **只读**：打开两个 SQLite 均使用只读标志；对夹具发起写操作失败（或 `PRAGMA query_only` 为 1），证明不会写用户数据。
3. **忙碌降级**：数据库被写事务占用（`SQLITE_BUSY`）时返回上一次快照且 `stale:true`，不 panic、不抛 5xx。
4. **`DSH_HOME` 解析**：`DSH_HOME` 已设置时使用它，未设置或为空白时回退 `~/.dsh`（单测覆盖三种输入）。
5. **helper 路由**：
   - `GET /codex-bot/summary|bots|rooms|messages` 在浏览器来源缺少 token 时被拒绝；
   - 带正确 token 时返回 spec 约定字段；`limit` 非法返回 `400`；
   - 数据源未就绪时返回 `200` + `available:false` + `reason`，而不是 5xx。
6. **用量归属**：`usage.sqlite` 中同一 `session` 的记录能按 `owners` 归属到对应 Bot；`today` / `week` 两个窗口的 token 合计与夹具手算值一致。
7. **消息截断与顺序**：`/codex-bot/messages` 按 `seq` 倒序返回最近 N 条，`text` 截断至 2000 字符，预览截断至 60 字。
8. **注入面板单测**：`node --test assets/inject/codex-bot-panel.test.mjs` 通过（23 项），覆盖：锚点存在时挂载 / 锚点缺失时不报错、开合状态写入并读回 `localStorage["codex-bot.panel.collapsed"]`、空态、错误态（helper 不可达）、用户数据不使用 `innerHTML` 拼接、**桥可用时完全不碰 `fetch` 且数据照常渲染**、**桥返回 `{status:"failed"}` 时保持错误态**，以及 7 项原版外观断言（品牌行 / Bot 行 / 侧栏与聊天双列 / 用量统计卡 / 群聊成员簇 / 消息气泡 / 原版空态文案）。
9. **打包生效**：`cargo build --release` 成功后，二进制中包含 `codex-bot` 文案与面板脚本片段（`grep -a codex-bot target/release/claude-codex-pro.exe` 有命中）。
10. **真机**：
    - 用 CCP 启动器启动 Codex，左栏出现「codex-bot」入口，点击展开面板并显示本机真实 Bot / 房间 / 用量；
    - 数据源不可用时（未运行 DS-Bot 或把 `DSH_HOME` 指到空目录）显示未就绪文案与重试按钮，Codex 其余功能正常。
11. **工程门**：`cargo fmt --all -- --check`=0；`cargo test --workspace --no-fail-fast`=0；`npm --prefix apps/claude-codex-pro-manager run check`=0；`npm --prefix apps/claude-codex-pro-manager run vite:build`=0；`cargo build --release`=0。
12. **缺失分区降级**：夹具只给 `usage.sqlite`（不给 `chat-index.sqlite`）时，`/codex-bot/summary` 仍为 `available:true`，但 `chatIndexAvailable:false`、`usageAvailable:true`；`/codex-bot/messages` 返回 `{available:false, reason:"missing-database"}`，且不返回 5xx。
13. **WAL 只读**：夹具里的 `usage.sqlite` 以 WAL 模式创建（同目录存在 `-wal`），只读查询仍能取到已提交数据；查询路径不得因尝试创建 `-shm` 而失败。
14. **真实形状**：夹具的 Bot 对象**不含 `workDir`**、`rooms` 为空、`usage` 表为 0 行时，各接口仍返回成功（`botCount:1`、`roomCount:0`、用量为 0），前端对应渲染为单 Bot / 房间空态 / 用量 0，不出现 NaN 或空白。

15. **真机只读冒烟**：`CCP_CODEX_BOT_REAL=1 cargo test -p claude-codex-pro-core --test codex_bot real_machine_snapshot_is_read_only -- --ignored --nocapture` 通过（本机实测 `available=true revision=3 bots=1 rooms=0 chatIndex=false(missing-database) usage=true`），且事后 `~/.dsh/bot` 下的文件未被改写。

16. **桥取数（页面 CSP 下的唯一可行通道）**：
    - `crates/claude-codex-pro-core/src/routes.rs` 的 `handle_bridge_request` 对 `/codex-bot/*`（含查询串）返回与 helper HTTP body 相同的 JSON；
    - 面板在 `window.__codexSessionDeleteBridge` 存在时只用桥（单测断言 `fetch` 调用数为 0），桥缺失时才直连 helper；
    - 真机：CCP 启动器注入后，Codex 页面内 `window.__codexBotPanel.snapshot().phase === "ready"`，且 `summary` 显示本机真实 Bot；同一页面直连 helper 的 `fetch` 仍失败（CSP 证据）。

17. **入口在左侧图标列**：真机里 `#codex-bot-nav-item` 的 `getBoundingClientRect()` 落在 Codex 左侧图标列内（`left ≤ 40`、`width/height ≤ 44`），其父链包含图标列容器（`group/nav-list flex flex-col gap-2` / `group/sidebar-rail`），且位于该列最后一个原生图标之后；不得出现在展开侧栏的会话列表里，也不得出现在左上角前进/后退一行。
18. **原版界面复刻**：面板视觉与文案取自 DS-Bot 原版客户端源码（`D:\DS-Bot-main\ds-bot\src\client\**`），不是自行设计；验收要点：
    - 结构：左侧 `bt-side` 列（品牌行 `bt-brand`、刷新、返回聊天 pill）与右侧聊天列双列布局；Bot 行（`.bt-row` + 墨色圆标 + 角色 chip）、群聊行（`.bt-cluster` + `N 位成员`）、消息气泡（`.bt-bubble` / 用户侧 `.bt-ububble`）与时间分隔（`.bt-time-sep`）、用量统计卡（`.bt-us-stats` / `.bt-us-card`）；
    - 文案沿用原版词汇（`主 Bot`、`Bot 列表`、`群聊`、`最近消息`、`N 位成员`、`新对话`）；
    - 注入的 CSS 只作用于面板自身（除 `#codex-bot-nav-item*` 外全部以 `#codex-bot-view` 为前缀），不覆盖 Codex 原生控件；不使用 `innerHTML` 渲染用户数据；
    - 明暗两种主题下都能完整渲染（真 Chromium 预览截图 + 真机截图各一份）。
    - 允许的非 1:1 项须在 spec 的「已知取舍」中列明并说明理由（字符 SVG 吉祥物、房间成员叠图、设置页脚/图标栏模式、完整用量页图表、输入框与工具行、原版 `返回对话` 文案）。

## 必需验证方式

- 命令：上面每条列出的命令；Rust 夹具在测试内以临时目录生成（`state.json` + 两个 SQLite），不提交二进制夹具。
- 证据：命令原始输出（通过 / 失败行数与 `test result`）、注入脚本单测输出、`grep -a codex-bot` 输出、真机截图（含入口、面板、未就绪态三张）。
- 手动：真机两条路径（有数据源 / 无数据源）各走一次，并确认 Codex 原生功能未被破坏；另用本机真实数据（1 Bot / 0 房间 / 缺 `chat-index.sqlite`）走一次，确认四类空态正常。

## 已知取舍

- v1 只读：不能从面板发消息；写路径依赖 DSH 宿主在线与 `state.lock` 单写者。
- 主键与 Bot 名称来自 `state.json`；DSH 升级若改变 schema，按 `unsupported-schema` 降级展示，不猜测字段。
- 面板只挂在左栏锚点上；Codex 若改版选择器，面板退化为不显示，不影响原生 UI。
- 未打包 Tailwind：ReUI / AI Elements 的交互与视觉只取结构、状态与 tokens，用纯 DOM + CSS 变量实现，避免为单个面板引入前端构建链。
- 本机 `chat-index.sqlite` 缺失属于数据源现状，不是本功能的缺陷；按分区降级处理并在面板上说明。

## 非目标

- MCP 包装、MCP Apps 或 Codex 官方插件市场发布。
- 修改 DS-Bot 插件源码、DSH 本体、Codex 安装文件。
- 远程 / 多机访问，或向非 Codex 客户端开放 `/codex-bot/*`。
- 从面板发起任何写操作（发消息、改配置、启停 Bot）。

## 执行记录（2026-10-28）

| 判据 | 命令 / 操作 | 结果 |
| --- | --- | --- |
| 8 | `node --test assets/inject/codex-bot-panel.test.mjs` | `# tests 16 / pass 16 / fail 0`（含桥两条：桥存在时 `fetch` 调用数为 0、桥返回 failed 时保持错误态） |
| 4/5/6/7/12/13/14 | `cargo test -p claude-codex-pro-core --test codex_bot` | `14 passed; 0 failed; 1 ignored` |
| 5/12/13/14 | `cargo test -p claude-codex-pro-core --lib codex_bot` | `7 passed; 0 failed` |
| 15 | `CCP_CODEX_BOT_REAL=1 … --test codex_bot real_machine_snapshot_is_read_only -- --ignored` | 通过（`available=true revision=3 bots=1 rooms=0 chatIndex=false(missing-database) usage=true`） |
| 16 | 真机（CCP 启动器注入，debug port 9230） | 面板 `version "2"`、入口 `#codex-bot-nav-item`、视图 `#codex-bot-view` 均存在；`api.refresh()` 后 `phase="ready" botCount=1 roomCount=0 revision=3`；启动器日志出现 `bridge.request` 的 `/codex-bot/summary`、`/codex-bot/bots?limit=20`、`/codex-bot/rooms?limit=20` 且同进程 `bridge.response` 应答 |
| 16（反证） | 同页面 `fetch(helper)` / 带 token / `mode:"no-cors"` | 三者均 `Failed to fetch` → 页面 CSP 阻断直连，桥为唯一可行通道 |
| 9 | `grep -a codex-bot target/release/claude-codex-pro.exe` | 有命中；产物 28,386,304 字节 @2026-10-28 22:44:04，已安装到 `H:\Claude Codex Pro`（旧文件备份为 `.bak-20261028-224626`） |
| 11 | `cargo fmt --all -- --check` / `cargo test --workspace --no-fail-fast` / `npm run check` / `vite:build` / `cargo build --release` | 全 0（workspace 43 个测试目标全部 `test result: ok`） |

已知取舍（本次新增）：桥与 HTTP 走同一份 `handle_route`，两条通道的返回体逐字节一致；桥缺失（手工注入、单测环境）时自动回退直连并带 token。

## 执行记录（2026-10-29 · 入口位置 + 原版界面）

| 判据 | 命令 / 操作 | 结果 |
| --- | --- | --- |
| 8 | `node --test assets/inject/codex-bot-panel.test.mjs` | `# tests 23 / pass 23 / fail 0`（原 16 项未改动；新增 7 项原版外观断言） |
| 8 | `node --check assets/inject/codex-bot-panel.js` | 通过（该文件被 `assets.rs:15` `include_str!`，解析错误会直接破坏注入） |
| 17 | 真机 CDP（debug port 9230，现场注入 v4 源码后测量） | `#codex-bot-nav-item` = `{x:17, y:228, w:18, h:18}`，父节点 `group/nav-list flex flex-col gap-2 …`，祖父节点 `group/sidebar-rail relative flex min-h-0 shrink-0 flex-col items-cente…` → 落在左栏图标列、紧接原生 `⋯` 图标之后；对照用户红框（x≈14–45、y≈218–268）一致 |
| 17 | 真机截图（原生聊天视图，入口可见） | `.dsh-ops/screenshots/codex-bot-rail-v4.png` |
| 18 | 原版外观渲染（真 Chromium 1440×900，两种主题） | 亮色 `.dsh-ops/screenshots/panel-light.png`、暗色 `panel-dark.png`（生成器：`%TEMP%\ccp-panel-preview\`，仅作证据，不入库） |
| 18 | CSS 作用域审计（真机页面内统计注入规则） | 132 条注入规则中仅 `#codex-bot-nav-item*`（3 条）不以 `#codex-bot-view` 为前缀，未泄漏到 Codex 原生 DOM；无 `innerHTML`、无外链 URL |

原版外观的源码出处（复刻依据，均在 `D:\DS-Bot-main\ds-bot\src\client\`）：`sidebar.js:304` `.bt-side`、`:90-95` `.bt-row` 结构、`:118-122` RoomAvatar 行；`styles.js:118-153`（`.bt-side*` / `.bt-brand` / `.bt-round` / `.bt-head` / `.bt-pill`）、`:138-145`（`.bt-row*` / `.bt-chip`）、`:179` `.bt-bubble`、`:296` `.bt-hint`、`:594` `.bt-msg`、`:911` `.bt-time-sep`、`:1105-1106`（`.bt-urow` / `.bt-ububble`）；`cells.js:69-72` / `:290-291` / `:413` / `:520`（消息与时间分隔）、`usage-page.js:150-154` / `:180-181` / `:215-222`（用量统计卡与排行卡）；`inks.js:2-7` / `:20-22`（墨色与 LEGACY_INKS）、`characters.js:39` `MAIN_LOOK`、`mark.js:51` `BotMark` / `:138` `RoomAvatar` / `:157` `.bt-cluster`；`i18n.js:9` `主 Bot`、`:16` `新对话`、`:17` `返回对话`、`:48` `N 位成员`、`:60` `Bot 列表`、`:61` 设置中提示。

非 1:1 项与理由已写入 [spec/codex-bot-injection-panel.md](spec/codex-bot-injection-panel.md) 的「已知取舍」。

## 执行记录（2026-10-29 · 底色不透明化与真机截图）

| 判据 | 命令 / 操作 | 结果 |
| --- | --- | --- |
| 8 | `node --test assets/inject/codex-bot-panel.test.mjs` | `# pass 23 / # fail 0`（新增 2 条断言：亮色与暗色都必须先铺实色底、再把宿主 token 叠上去） |
| 17 | 真机 CDP 计算样式 | 修复前 `#codex-bot-view` 计算底色 `color(srgb 0.964706 0.964706 0.964706 / 0.92)`（宿主 token 半透明 → 原生界面透出）；修复后 `rgb(252, 252, 252)`，`linear-gradient(color(srgb … / 0.92))` 叠在实色之上 |
| 17 | 真机截图（原生聊天视图，入口可见） | `.dsh-ops/screenshots/codex-bot-1-native-with-entry.png`、`.dsh-ops/screenshots/codex-bot-2-rail-closeup.png`（入口在左栏图标列、紧接原生 `⋯` 之后，`{x:17, y:228, w:18, h:18}`） |
| 18 | 真机截图（bot 视图） | `.dsh-ops/screenshots/codex-bot-3-bot-view.png`、`.dsh-ops/screenshots/codex-bot-4-bot-view-crop.png`（1280×1392；`version "5" phase "ready" botCount 1 roomCount 0 revision 3`） |
| 9 | 构建 + 安装 + 重启 | `cargo build --release` → 28,676,096 字节 @2026-10-29 01:11；安装到 `H:\Claude Codex Pro\claude-codex-pro.exe`（旧文件备份 `.bak-<stamp>`）并同步 `claude_codex_pro_manager_lib.dll`；用 CCP 启动器重启（`--launcher --debug-port 9230 --helper-port 53258`）后复测通过 |