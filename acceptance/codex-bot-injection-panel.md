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
8. **注入面板单测**：`node --test assets/inject/codex-bot-panel.test.mjs` 通过（16 项），覆盖：锚点存在时挂载 / 锚点缺失时不报错、开合状态写入并读回 `localStorage["codex-bot.panel.collapsed"]`、空态、错误态（helper 不可达）、用户数据不使用 `innerHTML` 拼接、**桥可用时完全不碰 `fetch` 且数据照常渲染**、**桥返回 `{status:"failed"}` 时保持错误态**。
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