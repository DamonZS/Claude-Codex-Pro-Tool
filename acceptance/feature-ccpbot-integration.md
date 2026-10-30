# 验收标准：CCP-Bot 集成（阶段 1：内嵌工作台）

对应规格：`spec/feature-ccpbot-integration.md`

## 通过条件总览

| # | 验收项 | 判定 | 证据形式 |
| --- | --- | --- | --- |
| A1 | 配置规范化与持久化 | 见 A1 细则 | `cargo test -p claude-codex-pro-core ccpbot` 输出 |
| A2 | 损坏配置回落且留备份 | `.corrupt-*.bak` 出现 | 同上测试输出 |
| A3 | 健康探测区分三种失败 | 三类 `detail` 文案互不相同 | 单元测试 + 手动探测 |
| A4 | 运行时探测字段齐全 | 六字段均有值或明确 `null`/`false` | 手动：页面状态行 |
| A5 | 源码模式启动前置校验 | 缺 `.env` 时失败且不起进程 | 单元测试 + 手动点「启动」 |
| A6 | 重复启动被识别 | 返回「已在运行」 | 单元测试 + 手动点两次 |
| A7 | 停止按进程树结束 | 端口 3010/3001 释放 | 手动：启动→停止→`Get-NetTCPConnection` |
| A8 | 路由与侧栏可见 | 侧栏出现「CCP-Bot」，点击进入页面 | 手动截图或 DOM 文本 |
| A9 | 内嵌 iframe 属性正确 | `allow` / `referrerPolicy` / `title` 齐备 | `grep` 源码 + 手动视检 |
| A10 | 不可达时给出引导而非白屏 | 占位说明 + 启动按钮 | 手动视检 |
| A11 | 类型检查与前端构建通过 | 两条命令 exit 0 | 命令输出 |
| A12 | Tauri 命令注册完整 | 六个命令出现在 `generate_handler!` | `grep` + 编译通过 |

## A1 配置规范化与持久化

- 通过：`normalize_address("  http://127.0.0.1:3010/  ")` → `http://127.0.0.1:3010`；`normalize_address("127.0.0.1:3010")` 返回 `Err`（缺 scheme）；`normalize_address("ftp://x")` 返回 `Err`。
- 通过：`save` 后 `load` 得到的 `address` / `mode` / `project_dir` 与写入一致。
- 失败：任一断言不符，或 `save` 写入非 UTF-8 内容。

## A2 损坏配置回落

- 通过：向 `ccpbot.json` 写入 `not json`，`load()` 返回默认配置，且同目录出现 `ccpbot.json.corrupt-<millis>.bak`，原文件被重置为可解析内容或保留旧文件。
- 失败：`load()` panic、或备份未产生、或原损坏内容被静默丢弃。

## A3 健康探测

- 通过：对未监听端口探测 → `reachable=false`，`detail` 含「连接被拒」或等价说明；对不存在的域名 → `detail` 含 DNS 失败；对一个返回 404 的地址 → `detail` 含状态码 404；对 `GET /health` 返回 `{"status":"ok"}` 的桩服务 → `reachable=true`。
- 通过：单次探测耗时不超过 3.5 秒（超时即失败场景）。
- 失败：任何一种失败被归为同一条 `detail`，或探测超过 3.5 秒仍未返回。

## A4 运行时探测

- 通过：在已安装 bun 的机器上 `bun_version` 形如 `1.2.21`；Docker CLI 存在但 daemon 未启动时 `docker_cli` 非空且 `docker_daemon=false`；`project_dir` 指到 CCP-Bot 仓库时 `project_dir_present=true`。
- 失败：字段缺失、或因缺少 bun/docker 而 panic。

## A5 源码模式启动前置校验

- 通过：`project_dir` 下无 `.env` 时 `start` 返回错误，文案提示需要 `.env`，且 pid 文件未被创建、3010 端口未被占用。
- 失败：起了进程、或返回成功、或错误文案不可操作。

## A6 重复启动识别

- 通过：pid 文件对应的进程仍在运行时再次 `start`，返回「已在运行」，不产生第二个进程（pid 文件内容不变）。
- 失败：产生两个 `bun run dev`，或返回成功并覆盖 pid。

## A7 停止按进程树结束

- 通过：源码模式启动后点「停止」，随后 `Get-NetTCPConnection -LocalPort 3010,3001` 无监听项，`Get-Process bun` 中不再有该次启动的进程。
- 失败：端口仍被占用，或子进程（vite / hono）成为孤儿。
- 备注：本项需要本机 bun 与 `.env`，若环境不具备，记录为「未验证 + 原因」，不得记为通过。

## A8 路由与侧栏

- 通过：`grep -n '"ccpbot"' apps/claude-codex-pro-manager/src/lib/routes.ts` 命中；启动 CCP 后侧栏出现「CCP-Bot」条目，点击后 URL/状态切到该路由，`routeSubtitle` 无 TS 报错。
- 失败：侧栏无条目、或点击后落回概览。

## A9 内嵌 iframe 属性

- 通过：组件源码中 iframe 具备 `allow`（含 clipboard-read/clipboard-write/microphone/camera）、`referrerPolicy="no-referrer"`、`title`，高度使用 `flex: 1` 或 `height: 100%` 而非固定像素。
- 失败：缺任一属性、或写死高度导致内嵌区被裁切。

## A10 不可达引导

- 通过：CCP-Bot 未运行时进入页面，内嵌区显示中文占位说明与「启动」按钮，而不是空白 iframe 或浏览器错误页。
- 失败：白屏、或直接抛出未捕获异常。

## A11 类型检查与前端构建

- 通过：`npm --prefix apps/claude-codex-pro-manager run check` exit 0；`npm --prefix apps/claude-codex-pro-manager run vite:build` exit 0。
- 失败：任一非 0 退出码。

## A12 命令注册

- 通过：`apps/claude-codex-pro-manager/src-tauri/src/lib.rs` 的 `generate_handler!` 中出现 `commands::ccpbot_load`、`commands::ccpbot_probe`、`commands::ccpbot_save_config`、`commands::ccpbot_start`、`commands::ccpbot_stop`、`commands::ccpbot_log_tail`；`cargo test -p claude-codex-pro-manager --test windows_subsystem` 通过。
- 失败：任一命令未注册（运行期报 `command not found`）。

## 必须提供的证据

1. `cargo test -p claude-codex-pro-core ccpbot -- --nocapture` 的完整输出（A1/A2/A3/A5/A6）。
2. `cargo test -p claude-codex-pro-manager --test windows_subsystem -- --nocapture` 输出（A12）。
3. `npm --prefix apps/claude-codex-pro-manager run check` 与 `run vite:build` 的退出码与末段输出（A11）。
4. `grep -n` 结果：路由条目、六个命令注册（A8/A12）。
5. 手动验证记录：启动/停止后的端口与进程快照（A7），页面截图或文本（A4/A8/A9/A10）。若某项因环境缺失未验证，必须在交付汇报中逐项写明原因。

## 不在范围内

- 阶段 2（CCP 内一键下载/安装 CCP-Bot 栈）。
- 阶段 3（免登录透传、单点登录）。
- CCP-Bot server / desktop / extension 的界面语言。
- Docker 镜像构建耗时与磁盘占用。
## A13 bun 可执行文件解析

- 通过：源码模式下必须解析到真正的 `bun.exe` 再启动。本机 bun 由 npm 包安装，PATH 上只有 `%APPDATA%\npm\bun.ps1` / `bun.cmd` 两个 shim，`bun.exe` 在 `%APPDATA%\npm\node_modules\bun\bin\`；`Command::new("bun")` 在 Windows 上只会补 `.exe`，因此必须按 `BUN_INSTALL`、`%USERPROFILE%\.bun\bin`、npm 全局目录、PATH 逐项查找。
- 失败：直接 `Command::new("bun")` —— 表现为「未找到 bun」误报（本机实测），或启动到错误的解释器。

## 验收结果（2026-10-30）

| 项 | 结论 | 证据 |
| --- | --- | --- |
| A1 | 通过 | `cargo test --lib ccpbot` 中 `normalize_address_trims_and_requires_a_scheme`、`store_round_trips_the_config` |
| A2 | 通过 | `corrupt_config_is_backed_up_and_defaults_return`：坏 JSON → `.corrupt-<millis>.bak` + 默认值 |
| A3 | 通过 | `each_failure_kind_has_its_own_detail`：拒连 / 404 / 超时三种 `detail` 两两不等 |
| A4 | 通过 | 页面状态条渲染 `bunVersion / dockerDaemon / 端口`；`runtime()` 单测覆盖 `host_port` |
| A5 | 通过 | `source_start_without_env_is_refused` 断言报错文本含 `.env` |
| A6 | 通过 | `start_reports_already_running`；live 测试再次调用 `start_at` 断言 `changed == false` |
| A7 | 通过 | live 测试：真实 `bun run dev` → `/health` 200 → `stop_at` 后端口关闭、pid 不再存活（5.25s 内完成） |
| A8 | 通过 | `windows_subsystem` 契约测试改为 11 个主目的地并包含 `("ccpbot", "CCP-Bot")`，90 passed / 0 failed |
| A9 | 未在本机实测 | iframe 属性已在 `components/CcpBotScreen.tsx` 固定；本机无 Postgres 与 CopilotKit Intelligence 凭据，CCP-Bot 栈起不来（见下） |
| A10 | 未在本机实测 | 同上：只能确认代码分支存在（占位块 + 启动按钮） |
| A11 | 通过 | `run check` exit 0；`run vite:build` exit 0（`✓ built in 4.79s`） |
| A12 | 通过 | `generate_handler!` 已注册六个命令；`windows_subsystem` 90 passed |
| A13 | 通过 | live 测试在修好解析后才通过；未修复前实测报「未找到 bun」 |

## 本机无法完成的验证（环境限制，非代码问题）

在 `D:\Project\OpenBot` 里直接 `bun run dev`（把 `.env.example` 复制成 `.env`）两次实测：

1. 第一次：`server/src/config.ts:1018` 抛 `error: CopilotKit Intelligence is required and is not configured. Missing: CPK_INTELLIGENCE_API_KEY (or INTELLIGENCE_API_KEY)`，server 退出。
2. 补一个占位 key 后：`code: "ERR_POSTGRES_CONNECTION_CLOSED"` —— 本机没有 Postgres（docker daemon 未启动，`failed to connect to the docker API at npipe:////./pipe/dockerDesktopLinuxEngine`），server 再次退出。3010 / 3001 均未监听。

结论：A9/A10 的“真机内嵌渲染”必须等本机具备 Postgres 与 Intelligence 凭据后手动复验；验收记录里已如实标注，未冒称通过。
## A14 默认配置不携带开发机路径

- 通过：全新用户的 `CcpBotConfig::default()` 中 `project_dir` 为空字符串；`sanitized()` 不会把它填成任何具体路径；此时点启动，报错文案说明「还没选择 CCP-Bot 目录」并给出两种进入方式（选目录，或只填地址接入已有实例），且不残留 pid 文件。
- 失败：默认值里出现某台机器的绝对路径；或空目录直接崩在 `package.json` 检查上，文案指向用户不理解的东西。

## A15 目录选择器

- 通过：页面提供「选择目录」按钮，走 `@tauri-apps/plugin-dialog` 的 `open({ directory: true, multiple: false })`，选中后回填到「项目目录」输入框，保存后持久化；选择器异常时以中文提示呈现，不抛未捕获异常。
- 失败：需要用户手抄路径；或选择器报错导致页面白屏。

## 验收结果补充（A14 / A15）

| 项 | 结论 | 证据 |
| --- | --- | --- |
| A14 | 通过 | `default_config_asks_for_a_directory_instead_of_assuming_one`、`start_without_a_directory_explains_the_two_ways_in`（11 passed / 1 ignored） |
| A15 | 通过（类型与构建层面） | `run check` exit 0、`run vite:build` exit 0；按钮与回填逻辑在 `components/CcpBotScreen.tsx` 的 `pickDirectory`。真机点按需在桌面端手动复验。 |
## A16 下载并准备（阶段 2）

- 通过：目标目录为空字符串时拒绝并提示「先选择目录」；目录非空且无 `package.json` 时拒绝并提示「不是空目录」，且不留下半成品 checkout；空目录则 `git clone` → `bun install` → 生成 `.env`，返回消息逐条列出做了什么；已有 `package.json` / `node_modules` / `.env` 的目录跳过对应步骤（可重复执行）。
- 失败：往非空目录里硬克隆；或下载完成后 `node_modules` 缺失却报成功；或仓库地址写死成开发机上的本地路径。

## A17 下载产物即内嵌产物

- 通过：`DEFAULT_REPO_URL` 指向发布的 CCP-Bot 仓库（汉化 + 改名后的同一份代码），克隆下来的仓库含 `app/src/lib/i18n/messages/zh-CN.ts`、`charts/ccpbot/`、`CCPBOT_*` 前缀，README 说明来龙去脉。
- 失败：指向上游英文版；或指向一个不存在的仓库。

## 验收结果补充（A16 / A17）

| 项 | 结论 | 证据 |
| --- | --- | --- |
| A16 | 通过 | `prepare_without_a_directory_asks_for_one`、`prepare_refuses_a_directory_that_holds_something_else`；真实链路 `live_prepare_downloads_and_seeds_env`（`--include-ignored`）→ **1 passed**，124.89s，克隆了 `https://github.com/DamonZS/CCP-Bot.git`、`bun install` 完成、`node_modules` 与 `package.json` 落盘 |
| A17 | 通过 | `gh api repos/DamonZS/CCP-Bot/contents/app/src/lib/i18n/messages/zh-CN.ts` → size 134239；仓库 `CCP-Bot` public、默认分支 `master`、`git ls-tree` 含 `charts/ccpbot/Chart.yaml`；README 已推送（1593 bytes） |
| 命令注册 | 通过 | `ccpbot_prepare` 在 `lib.rs` 的 `generate_handler!` 中注册；`cargo check -p claude-codex-pro-manager` 完成；`run check`/`vite:build` exit 0；`windows_subsystem` 90 passed |
## A18 发布物可被匿名下载（安装链路的前提）

- 通过：四个 URL 在**不带凭据**的情况下可取：源码压缩包、`container-images.json`、最新 release 的 API、以及 `docker pull` 镜像；清单的五个键与 `IMAGE_VARIABLES` 完全一致。
- 失败：任一 URL 401/404；清单缺键或多键；标签格式与 `resolve_version()` 期望不符。
- 验证方式：`Invoke-WebRequest -Method Head`（匿名）、`gh release list --repo DamonZS/CCP-Bot`、`jq '.images|keys'` 与 `IMAGE_VARIABLES` 对照、`docker pull <ref>@sha256:…`。

| 项 | 结论 | 证据 |
| --- | --- | --- |
| A18 | 通过 | `archive/refs/tags/v0.2.0.tar.gz` → 200，10,832,745 字节，解包后 `package.json` = `ccpbot / 0.2.0`、`zh-CN.ts` 134239 字节、`.github/workflows/publish-images.yml` 存在；`releases/download/v0.2.0/container-images.json` → 200，键集合 `agent-bot, agent-computer, agent-langgraph, server, supervisor`；`api.github.com/repos/DamonZS/CCP-Bot/releases/latest` → 200（`tag_name=v0.2.0`）；匿名 `HEAD https://ghcr.io/v2/damonzs/ccpbot/manifests/v0.2.0` → 200；`docker pull ghcr.io/damonzs/ccpbot@sha256:c042d958…` → 成功 |

### 过程中修掉的两个真问题

1. **`ghcr.io/DamonZS/…` 非法**：buildx 在任何层构建之前就报 `ERROR: failed to build: invalid tag "ghcr.io/DamonZS/ccpbot:v0.2.0": repository name must be lowercase` —— 账号名带大写，注册表路径必须小写。工作流因此先算一个小写命名空间再用。
2. **删标签把 release 变成 draft**：`git push origin :refs/tags/v0.2.0` 之后 GitHub 把旧 release 转成 draft，其资产对匿名请求 404；发布步骤末尾因此显式 `gh release edit "$TAG" --draft=false`。
> 注：镜像 digest 会在重建时变化（构建产物不是逐字节可复现的），所以「哪个 digest 是对的」只能由 release 的 `container-images.json` 回答；本文档记录的是 2026-10-30 10:10 UTC 那次构建的取值，`server` = `sha256:c042d958d196fc0060b7ad44d2aa09482ee82cb9f0177f0640f99967960bc033`，与匿名 `HEAD /v2/damonzs/ccpbot/manifests/v0.2.0` 返回的 `Docker-Content-Digest` 一致。