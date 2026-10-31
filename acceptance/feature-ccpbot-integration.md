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
## A19 集群安装（chart 渲染）指向已发布的镜像

- 通过：`helm template` 渲染出的 `image:` 是 `ghcr.io/damonzs/ccpbot:v0.2.0`；`--set image.digest=…` 后变成 `ghcr.io/damonzs/ccpbot:v0.2.0@sha256:…`；渲染结果里没有任何 `copilotkit` 字样。
- 失败：渲染出 `ghcr.io/copilotkit/…`；或 tag 为空/与 `appVersion` 不匹配；或渲染失败。
- 验证方式：`helm template release . …` + 对 `image:` 行做分组统计。
- 环境事实（复现要用的）：本机 helm 由 `winget install Helm.Helm` 装（`v4.3.1`）；`registry-1.docker.io` 直连超时，chart 依赖 `postgresql 16.7.27` 是从 `docker.1ms.run` / `docker.m.daocloud.io` 镜像 `helm pull oci://<mirror>/bitnamicharts/postgresql --version 16.7.27` 取到后放进 `charts/` 才渲染成功的（未提交该 tgz，也未改 `Chart.yaml` 的依赖行）。渲染必须补齐的必填值：`secrets.intelligenceApiKey`、`database.url`、`config.singleUser=true`、`config.intelligence.apiUrl`、`config.intelligence.gatewayWsUrl`、`secrets.computerToken`、`secrets.supervisorToken`、`secrets.workerToken`、`secrets.keyEncryptionKey`。

| 项 | 结论 | 证据 |
| --- | --- | --- |
| A19 | 通过 | `render ok, lines=489`；`4x image: ghcr.io/damonzs/ccpbot:v0.2.0`；补 `image.digest` 后 `image: ghcr.io/damonzs/ccpbot:v0.2.0@sha256:c042d958d196fc0060b7ad44d2aa09482ee82cb9f0177f0640f99967960bc033`（该 digest 取自 release 的 `container-images.json`，与匿名 `HEAD /v2/…/manifests/v0.2.0` 一致）；`copilotkit hits = 0` |

## A20 桌面安装器真的从 GitHub 把这一份下载下来

- 通过：桌面 crate 自带的 ignored 用例 `live_latest_release_is_downloaded_and_pinned` 对着真实 GitHub 跑通：取 latest release → 下载 tarball → 按 `REQUIRED` 列表铺开部署 → 取 `container-images.json` → 记录版本 → 重启动不再需要网络。
- 失败：下载失败、清单缺失、`package.json` 版本与 tag 不一致、或重启动仍访问网络。
- 验证方式：`cargo test --lib live_latest_release_is_downloaded_and_pinned -- --include-ignored --nocapture`（在 `desktop/src-tauri` 下）。

| 项 | 结论 | 证据 |
| --- | --- | --- |
| A20 | 通过 | `Finished test profile in 1m 12s`；`Downloaded and pinned v0.2.0; tagged source, image manifest, and offline reuse verified.`；`test result: ok. 1 passed; 0 failed`，用例自身耗时 5.98s |

### 版本状态说明

`v0.2.0` 标签指向 `d67e273`（CI 修好后重打的那次），其后的 `87d1604` 只改了 `charts/ccpbot/values.yaml` 的注释，不影响镜像与 tarball 内容；因此没有再次重打标签，避免 digest 反复变化让文档失真。
## A20 复验：下载链路只依赖能连通的三个主机

复跑时 `github.com`（网页/重定向主机）在本机持续超时（`curl: (28) Connection timed out after 12009 ms`，三次连续），而 `codeload.github.com`（200，10,832,793 字节，1.8s）与 `api.github.com`（200）都正常。原实现有两处都落在不可达的主机上，于是把下载改成只走后两者：

| 用途 | 之前 | 现在 |
| --- | --- | --- |
| 源码 tarball | `https://github.com/DamonZS/CCP-Bot/archive/refs/tags/v0.2.0.tar.gz`（302 → codeload） | `https://codeload.github.com/DamonZS/CCP-Bot/tar.gz/refs/tags/v0.2.0` |
| 镜像清单 | `https://github.com/DamonZS/CCP-Bot/releases/download/v0.2.0/container-images.json` | `https://api.github.com/repos/DamonZS/CCP-Bot/releases/tags/v0.2.0` → `releases/assets/<id>`（`Accept: application/octet-stream`） |

`desktop/src-tauri/src/deployment.rs` 的 `images_url` 因此改为返回 release JSON，新增 `release_asset(response, name)` 取 asset id、`get_asset(url)` 带 `Accept` 头；`fetch_images` 走这条两跳链路。清单文件本身仍是 `release` 里的 `container-images.json` 资产，格式未变，`pin()` 与 compose 变量生成都不受影响。

| 项 | 结论 | 证据 |
| --- | --- | --- |
| A20（改后复跑，**直连、无代理**） | 通过 | `cargo test --lib live_latest_release_is_downloaded_and_pinned -- --include-ignored --nocapture` → `Downloaded and pinned v0.2.0; tagged source, image manifest, and offline reuse verified.`；`test result: ok. 1 passed; 0 failed`，用例 5.56s |
| `deployment::` 单测 | 通过 | 21 passed / 0 failed（新增 `the_release_names_the_asset_the_id_points_at`：取 id、缺资产报错、非 JSON 报错） |

### 全量单测的 3 个既有失败（与本次改动无关，如实记录）

`cargo test --lib`（桌面 crate）→ **603 passed / 3 failed / 9 ignored**，失败项都不是本次改动引入：

1. `harness::tests::every_image_is_one_a_release_publishes` → `published-images.json is not where this test expects it`：这个 fork 的 `.github/` 下只有 `workflows/publish-images.yml`，**没有** `published-images.json`（`git ls-files .github` 只列出工作流）。该用例要读它来校验 `shared/harnesses.json` 里 13 个 harness 镜像都已发布；文件补上与否取决于是否真的发布那 13 个镜像（我们只发 5 个），属独立议题。
2. `provider_oauth::tests::credential_lock_excludes_bun_in_both_directions_and_recovers_after_kill` → `Error { kind: NotFound, message: "program not found" }`（用例要起 `bun`）。
3. `provider_oauth::tests::credential_replacement_waits_for_existing_rotation_owner` → `Set-Acl … PrivilegeNotHeld … SeSecurityPrivilege`（需要提升的 ACL 权限）。
### A21：一键安装最新版（`ccpbot_install_latest`）

验证对象：`crates/claude-codex-pro-core/src/ccpbot.rs` 的 `download_latest_installer()` 与 `install_latest()`、命令 `ccpbot_install_latest`、界面「安装最新版」按钮，以及产出安装包的 `.github/workflows/build-installers.yml`。

| # | 通过标准 | 证据 |
| --- | --- | --- |
| A21.1 | 按平台与架构选资产，**架构不对就拒绝** | 单测 5 条：`a_windows_install_prefers_the_silent_setup_over_the_msi`（`.exe` 优先于 `.msi`）、`a_windows_install_falls_back_to_the_msi`、`a_mac_install_takes_the_bundle_for_this_architecture`、`an_installer_for_the_other_architecture_is_refused_not_offered`（只发布了另一种架构时明说「装了也起不来」，而不是把 ARM 包装给 Intel Mac）、`a_release_without_an_installer_does_not_pretend_it_has_one`。`cargo test -p claude-codex-pro-core --lib ccpbot` → **19 passed / 0 failed / 3 ignored** |
| A21.2 | 网络那一半对着真实 release 跑通 | `cargo test -p claude-codex-pro-core --lib ccpbot::tests::live_latest_release_offers_this_machines_installer -- --include-ignored` → **1 passed**（3.84s）；落盘 `%USERPROFILE%\.claude-codex-pro\ccpbot-installers\CCP-Bot_0.2.3_x64-setup.exe` = **4,719,400 字节**，与 release 资产字节数一致；日志 `ccpbot-install.log` 记了 `releases/latest` → 资产 URL → 字节数 |
| A21.3 | 安装包真的被发布 | run **41063135835**（`workflow_dispatch`，tag `v0.2.3`）→ **success**：`windows-x64`、`macos-arm64`、`attach to the release` 三个作业全 success。首跑暴露两个真问题并都修了：空签名密钥被 Tauri 当成「给了证书」（`failed to import keychain certificate`，修成非空才注入，`c4e2721`）；`macos-13`（Intel）在两个 run 里各排队 40 分钟以上而别的作业早就完了，且它的产物在 Apple 芯片上根本跑不起来 —— 于是矩阵只剩 Apple 芯片（`d2263d2`） |
| A21.4 | 发布资产匿名可取 | `gh release view v0.2.3 --json assets` → `draft=false`，资产：`CCP-Bot_0.2.3_x64-setup.exe` 4,719,400 B、`CCP-Bot_0.2.3_x64_en-US.msi` 6,889,472 B、`CCP-Bot_0.2.3_aarch64.dmg` 9,550,930 B、`container-images.json` 3,063 B |
| A21.5 | 界面接线 | `npm --prefix apps/claude-codex-pro-manager run check`（tsc）无错；命令注册在 `apps/claude-codex-pro-manager/src-tauri/src/lib.rs` 的 `generate_handler!`（`commands::ccpbot_install_latest`，现 :102）；`cargo check -p claude-codex-pro-manager` 通过 |
| A21.6 | 不装系统的系统不硬撑 | Linux 上 `install_latest` 直接返回「一键安装只提供 Windows 与 macOS 的安装包；这套系统请用「下载并准备」从源码启动。」（`Platform::Unsupported` 分支 + macOS DMG 分支的「拖进应用程序」实测表述） |

不在范围：签名（Windows 证书 / Apple 开发者证书）只保留了读取 secrets 的能力，仓库里没有证书，因此安装包是**未签名**的：Windows 会显示「未知发布者」，macOS 需要清 quarantine 或右键打开。这是明说的代价，不是遗漏。
| A21.7 | 安装包真的能装（不是只有字节数） | 把资产拷到临时目录用 NSIS 的 `/D` 静默安装：`exit=0`，产出 `ccpbot-desktop.exe`（19,157,504 B）与 `uninstall.exe`；`VersionInfo` 为 `ProductName=CCP-Bot / FileVersion=0.2.3 / Company=ccpbot`；`uninstall.exe /S` → `exit=0`、残留 0 个文件。**按下按钮时走的是不带 `/D` 的同一条命令**，即静默装进默认位置（这就是「一键安装」的含义），临时目录只是验证方式 |

### A22：CCP-Bot 0.2.4 —— 标准账号不再被 Windows 前置关卡挡住

起点是一次真机反馈：装上 CCP-Bot 桌面端后，界面停在「CCP-Bot needs one thing first / This account cannot install Windows components」，要求用管理员执行两条 `dism.exe /online /enable-feature` 与 `wsl --install`。实测这台机器上其实有能跑这套栈的引擎（用户级安装的 Docker Desktop），所以这个画面是产品缺陷，不是环境缺陷。

| # | 通过标准 | 证据 |
| --- | --- | --- |
| A22.1 | 缺陷复现（修前判定） | `Test-Path "C:\Program Files\Docker\Docker\Docker Desktop.exe"` → **False**；`%LOCALAPPDATA%\Programs\DockerDesktop\Docker Desktop.exe` → **True**。旧代码（`desktop/src-tauri/src/docker_desktop.rs:44-50`）只查前者，于是 `installed_app() = None`，引擎阶梯跳过 Docker 落到 Podman（需要 WSL2 + VirtualMachinePlatform → 需要管理员）。同时 `vm_disks()` 里的 `%LOCALAPPDATA%\Docker\wsl\disk\docker_data.vhdx` 存在（True），说明这台机器早就跑过 Docker |
| A22.2 | 修后：真实机器上关卡不再升起 | 一次性探针（临时 integration test，跑完即删、不入库）：`cargo test --test gate_on_this_machine -- --nocapture` → `installed_app = Some("C:\Users\Damon\AppData\Local\Programs\DockerDesktop\Docker Desktop.exe")`、`used_before = true`、`saved_engine = None`、`docker_is_the_engine = true   (gate raised: false)`，`test result: ok. 1 passed` |
| A22.3 | 单测 | `cargo test --lib -- docker_desktop:: install::tests::docker_ windows::tests::` → **45 passed / 0 failed / 1 ignored**；`cargo test --lib -- a_machine_without_a_docker_that_has_run` → **1 passed**。新增 `docker_desktop::tests::both_windows_roots_are_places_docker_desktop_can_be`、`install::tests::docker_takes_the_windows_stage_off_the_path`、`install::tests::a_machine_without_a_docker_that_has_run_keeps_the_windows_stage`，并在 `windows::tests::assert_administrator_setup_instruction` 里断言被挡住时的文案提到 Docker Desktop |
| A22.4 | 全量单测 | `cargo test --lib` → **607 passed / 2 failed / 9 ignored**；两条失败是既有的 `provider_oauth` 用例（分别要 PATH 上的 `bun`、要 `SeSecurityPrivilege`），与本改动无关。`cargo check --bin ccpbot-desktop` → `Finished`（`windows_blocker` 编译在 bin 里，`--lib` 看不到它） |
| A22.5 | 这一版真的发出来了 | `publish-images` run **41162571159** → success（21/21 作业）；`build-installers` run **41162571163** → success；release `v0.2.4` `draft=false`，资产 `CCP-Bot_0.2.4_x64-setup.exe` **4,710,923 B**、`CCP-Bot_0.2.4_x64_en-US.msi` 6,885,376 B、`CCP-Bot_0.2.4_aarch64.dmg` 9,549,623 B、`container-images.json` 3,063 B（`version=v0.2.4`，19 个镜像带 digest）；master 上的 `CI` run **41162568768** → success |
| A22.6 | 「一键安装」取到的就是这一版 | `cargo test -p claude-codex-pro-core --lib ccpbot::tests::live_latest_release_offers_this_machines_installer -- --include-ignored` → **1 passed**（1.65s）；`%USERPROFILE%\.claude-codex-pro\ccpbot-install.log` 记 `GET https://api.github.com/repos/DamonZS/CCP-Bot/releases/latest` → `GET https://github.com/DamonZS/CCP-Bot/releases/download/v0.2.4/CCP-Bot_0.2.4_x64-setup.exe` → `saved 4710923 bytes`，落盘文件字节数与资产一致 |

不在范围：既没有管理员权限、也没有任何现成引擎的机器仍需要一次管理员操作（Podman 路线要 WSL2 与 VirtualMachinePlatform，这是系统本身的要求）；这次解决的只是「已经装了 Docker Desktop 却被告知没有引擎」那一类。安装包仍未签名（Windows 显示未知发布者，macOS 需清 quarantine）。

### A23：CCP-Bot 界面重做——一屏一个主角

起点是一次真机截图的评价（「太丑了」）：旧页面把八颗按钮、三个输入框和两条提示全堆在内嵌界面上面，未运行时舞台是一片黑，控件就是全部内容。重做按 `F:\学习\AI\高级交互动效GPT提示词手册（51条完整版）.md` 的规范执行，取用 #1 按压回弹、#2 按压瞬间反馈、#6 成功勾选、#10 骨架屏落位、#19 动态弥散光晕、#20 物理弹簧交错流、#21 玻璃面板浮起、#28 Toast 进出场、#38 暗黑科技风 Dashboard，以及总则「一屏一个主角 / 留白即高级 / 一屏一个高亮色 / 组件统一」。

| # | 通过标准 | 证据 |
| --- | --- | --- |
| A23.1 | 主角是内嵌界面，不是控件 | 顶部一条状态栏（脉冲状态球、地址、等宽读数、启停分段、图标操作），下面整块舞台；8 颗按钮与 3 个输入框退出常驻布局：启停 2 颗留在状态栏、其余进「设置」面板与离线清单 |
| A23.2 | 状态栏如实反映机器 | 球三态：可达=荧光绿脉冲、启动中/加载中=琥珀呼吸、其它=灰；读数只有 `probe.status == 200` 才给等宽绿数（本机实测 Docker 会答 503，那时它显示「已应答 503」，不是编出来的「在线」）；`docker daemon 未启动` 这类事实照实显示 |
| A23.3 | 离线舞台给出路线而不是空白 | 三步清单「取得 CCP-Bot／本机 bun 或 Docker 引擎／配置（.env）」，每步一句实话 + 一键操作（安装最新版／下载并准备／选择目录），并把 `probe.detail` 完整呈现（`连接被拒：该地址没有服务在监听…`）；此后场景切换为 24px 网格点纹理与两级投影，不再是「什么都没有」 |
| A23.4 | 布局真的撑满（这是重做的关键修复） | 高度链本来断在 `.ops-page-content`（它没有高度，于是 `.ccpbot-layout` 的 `height:100%` 解析成 auto，舞台塌成中间一条带）。改为路由专属规则：`.ops-screen[data-route="ccpbot"]{display:flex;flex-direction:column;min-height:0;overflow:hidden;padding:12px 16px 14px}` + `> .ops-page-content{display:flex;flex:1 1 auto;min-height:0}` + `.ccpbot-layout{flex:1 1 auto;height:auto}`；并隐掉重复的通用页标题 `.ops-page-heading`（状态栏已经写着产品名与地址） |
| A23.5 | 两套主题都成立 | 全部颜色走既有 HSL token（`--surface-sunken/--surface-raised/--border/--brand-accent/--destructive`），网格纹理用 `hsl(var(--foreground) / 0.045)`（白线叠加在浅色面上不可见，必须按主题取反）；深浅两张截图各自成立 |
| A23.6 | 通知不再挤动布局 | 段落式提示改为右上角浮动 `.ccpbot-toast(.ok/.warn)`（进场 `cubic-bezier(0.34,1.56,0.64,1)`、退出 240ms、可叠加），日志收进底部抽屉 `<details>` |
| A23.7 | 命令契约未动 | 组件仍只调 `@/lib/ccpbot` 的七个包装（`load/probe/save/start/stop/log_tail/prepare/install`）+ 既有的 `open_external_url` 与目录选择器；Rust 侧与 `acceptance` A1–A22 的命令面零改动 |
| A23.8 | 静态验证 | `npm --prefix apps/claude-codex-pro-manager run check`（tsc）exit 0；`npm run vite:build` → `✓ built in 2.07s`（仅既有的 chunk>500kB 警告）。前端测试运行器这次补上了，见 A23.11 |
| A23.9 | 真机 Tauri 窗口证据（这次补上） | 后台 `npm run dev`（`tauri dev`，job `pwsh-99d88e2f844a4745`）拉起真窗口 `CCP 管理工具`（1196×859），逐张实拍：`%TEMP%\ccpbot-app-overview.png`（1,089,524 B，启动落在概览页）、点侧栏「CCP-Bot」后 `%TEMP%\ccpbot-app-page.png`（640,654 B，**骨架态**：`loading` 期间三根占位条，说明真后端 `ccpbot_load` 比首帧慢）、约 25 秒后 `%TEMP%\ccpbot-app-settled.png`（658,846 B，读数 `离线 / Docker / 1.2.21 / 未启动`、三步清单、探测原文逐字上屏）、点状态栏最后一颗图标 `%TEMP%\ccpbot-app-log.png`（634,388 B，日志抽屉真的展开、正文「暂无日志。」） |
| A23.10 | 预览环境能复现这三种状态 | `apps/claude-codex-pro-manager/src/tauriPreviewMock.ts` 新增 `ccpbot_*` 模拟（默认=全新机器：有 bun、无目录、不可达；`ccpbot_start` 翻成可达并给 pid），因此无 Tauri 的浏览器里可以评审；离线/在线两张图即为该路径产出（在线与失败态是临时翻转 mock 后取样，取样完已还原，`Select-String '临时截图|4172 as number'` 命中 0） |
| A23.11 | 前端测试门禁（这次补上） | 装 `vitest 5.0.5`、`@testing-library/react 16.3.3`、`@testing-library/dom 10.4.2`、`jsdom 29.1.1` 为 devDependencies，并同步 `apps/claude-codex-pro-manager/package-lock.json`（CI 用 `npm ci`，不同步会直接挂）；`vite.config.ts` 的 `defineConfig` 改从 `vitest/config` 引入、加 `test: { environment: "jsdom", include: ["src/**/*.test.{ts,tsx}"], restoreMocks: true, setupFiles: ["./src/test/setup.ts"] }`（契约测试只断言 `base: "./"`，该行保留）；`package.json` 加 `test` / `test:watch`。3 个文件 16 条：`src/components/CcpBotScreen.test.tsx`(9)、`src/lib/ccpbot.test.ts`(4)、`src/lib/routes.ccpbot.test.ts`(3)，`npm run test` → **Test Files 3 passed / Tests 16 passed（4.42s）** |
| A23.12 | 测试抓到一个真 bug 并修掉 | `toggleLog` 原先调 `recheck()`，而 `recheck` 闭包捕获的 `logOpen` 还是旧值 —— **打开日志抽屉永远不会去读日志，抽屉永远空**。修法：新增 `readLog` 回调（`readCcpBotLog(120)` 写 `logPath/logText`，失败给 `读取日志失败：…`），`toggleLog` 在翻开时调它；A23.9 第四张真机截图就是这条路径 |
| A23.13 | 两条既有 CSS 契约逼着换纹理（记录在案） | 新舞台的网格原用 `repeating-linear-gradient` 与 `radial-gradient`，被 `apps/claude-codex-pro-manager/src-tauri/tests/windows_subsystem.rs:1936` 与 `:5235` 的 `!workspace.contains(...)` 判失败（当时 88 passed / 2 failed）。改为两层普通 `linear-gradient` + `background-size: 28px 28px` 平铺、光晕由径向改竖向渐变；连注释里也不能出现那两个函数名（同样会被 `contains` 命中）。改后契约测试 **90 passed / 0 failed**；空状态 44px 圆标改走 `--brand-accent`（浅色主题下 muted grey 在近白底上糊成一团）。换纹理后复拍 `%TEMP%\ccpbot-appearance-light.png`（581,214 B）、`%TEMP%\ccpbot-appearance-dark.png`（481,825 B） |
| A23.14 | 测试进了 CI | `.github/workflows/pr-build.yml` 与 `.github/workflows/auto-release-installers.yml` 中每个 `Type-check Manager` 之后各加一步 `Test Manager interface`（`run: npm run test`，共 4 处）；`node scripts/release/verify-release-workflow.js` → `10 NSIS installation scenarios passed.` + `release workflow contract passed`（exit 0），既有断言（`npm run check` ×2、`npm run vite:build` ×2、`macos-latest` ×1）未被触碰 |

不在范围：动效手感（曲线、时长、`prefers-reduced-motion` 处理都写进了 CSS 并随截图留证，但静态图证明不了手感）；真机内嵌态只到「离线页」——本机没有跑起来的 CCP-Bot 引擎（Docker daemon 未启动，也缺 Postgres 与 `CPK_INTELLIGENCE_API_KEY`），所以 iframe 里是浏览器的错误页而非真界面；macOS 与其它机器的观感未验证。
## A24：自动检测已装版本 + 一键安装（目录先选后装）

| 检查项 | 证据 |
| --- | --- |
| A24.1 | 检测信号是真机取数，不是猜的 | `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\CCP-Bot` 下带 `DisplayVersion = 0.2.4`、`InstallLocation`、`UninstallString`、`MainBinaryName = ccpbot-desktop.exe`、`Publisher = ccpbot`。把 0.2.4 的安装包以 `/S /D=%TEMP%\ccpbot-install-probe` 静默装出真实痕迹后逐项读出；再 `uninstall.exe /S` → **目录与注册表键一起消失**，同一探针第二次返回 `desktopPath=None / desktopVersion=None / uninstallPath=None`，只剩 `image=Some("ghcr.io/damonzs/ccpbot:v0.2.1")`（本机 kind 留下的一次 `kind load`）。⇒ 已装的人不会被误报、也不会漏报镜像 |
| A24.2 | 检测结果落到界面的三处 | `crates/claude-codex-pro-core/src/ccpbot.rs` 的 `installed()` 五路探测（注册表 `InstallLocation` + `%LOCALAPPDATA%\Programs\CCP-Bot` 等候选目录找 `ccpbot-desktop.exe`、`DisplayVersion`、`uninstall.exe`、源码目录的 `package.version`、`docker image ls` 里的镜像）→ 命令层把它塞进 `ccpbot_load`/`ccpbot_probe` 的 payload（`installed` + `defaultInstallDir`）→ 界面：状态栏多一枚「已装 0.2.4」读数、三步清单第一步的说明变成「桌面版已装 0.2.4」、空状态底部按是否已装切按钮 |
| A24.3 | 目录先问、再下载、最后真验 | 按钮「一键安装（可先选安装目录）」先弹目录选择器（`defaultPath` 就是后端给的 `%LOCALAPPDATA%\CCP-Bot`），取消也照样发给后端（`{ request: null }`，让安装器自己决定位置）；`installer_arguments()` 对 NSIS 把 `/D=<dir>` 放在**最后且不带引号**、对 MSI 用 `INSTALLDIR=<dir>` 属性；安装后**实测** `<dir>\ccpbot-desktop.exe` 是否落地：在则消息里附上路径，不在则明说「安装器可能没接受这个位置」，不谎报成功 |
| A24.4 | 已装的人不被推去重复下载 | 已装时按钮变「重新安装最新版」，`一键安装` 不再出现（前端测试断言 `queryByRole` 为 null） |
| A24.5 | 命令契约 | `commands::ccpbot_install_latest(request: Option<CcpBotInstallRequest>)`，参数名 `request`，`{ dir }` 或 `null`；`src/lib/ccpbot.ts` 的 `installCcpBotLatest(dir = null)` 负责这两种形态 —— 有单测钉住 |
| A24.6 | 测试 | Rust：`cargo test -p claude-codex-pro-core --lib ccpbot` → **24 passed / 0 failed / 4 ignored**（新增 6 条：NSIS 末尾参数、MSI 属性、`reg query` 整行读取不得前缀顶替、checkout 的版本、无版本目录不算 checkout、`live_install_honours_the_directory_it_was_given` 的 ignored 用例）；前端：`npm run test` → **19 passed**（`CcpBotScreen.test.tsx` 12 条含三条安装流程：问目录→装→读回版本、取消→`{ request: null }`、已装→显示版本不给一键安装）；`npm run check` exit 0；`npm run vite:build` exit 0（5.76s）；`cargo test -p claude-codex-pro-manager --test windows_subsystem` → **90 passed / 0 failed** |

不在范围：安装包仍未签名（SmartScreen/隔离警告照旧）；macOS 上目录语义不同（`.app` 是整包，选择器的结果交给 `open` 而不是就地安装）；安装器自己的向导文案不归我们；「已装但版本更旧」时不做自动升级（只在界面上显示版本并给「重新安装最新版」）。

## A25：CCP-Bot 0.2.5 起「装完就能用」（本机进程，不需要引擎）

CCP 这一侧不做安装本身，但必须知道 0.2.5 的安装包行为已经变了 —— 它决定「一键安装」之后用户还要不要再做一步。

| 检查项 | 证据 |
| --- | --- |
| A25.1 | 安装包默认不需要引擎 | CCP-Bot 0.2.5（`package.json` 0.2.5、`Chart.yaml` version 0.1.8 / appVersion 0.2.5、`CHANGELOG.md` 小节 `## 0.2.5 - it does not need Docker`）的桌面端在首次运行时下载便携 PostgreSQL（EDB `postgresql-17.6-1-windows-x64-binaries.zip`，329,891,687 B，SHA-256 钉住）与 pgvector 扩展包，起本机进程，不碰 Docker/WSL/虚拟机；发布资产 `CCP-Bot_0.2.5_x64-setup.exe` 4,769,521 B、`…_x64_en-US.msi` 6,971,392 B、`…_aarch64.dmg` 9,614,870 B（run 41185919931 / 41185919986） |
| A25.2 | 端到端真机证据在 CCP-Bot 仓 | `cargo test -p ccpbot-desktop --manifest-path desktop/src-tauri/Cargo.toml --lib -- local::tests::live_start --include-ignored --nocapture` → `1 passed … finished in 14.37s`，步骤 `[runtime]… → [database]… → [schema]… → [bot] The built-in Bot is answering (/health 200). → [server]… → [ready] CCP-Bot is running on this computer (/health 200).`，`local stack up on http://127.0.0.1:3010 with database port 63133 and the Bot on http://127.0.0.1:63659`；`logs/local-bot.log` 末行 `agent-bot listening on http://127.0.0.1:63659/ag-ui`。规格与四个真机坑写在 CCP-Bot 仓的 `docs/local-mode.md` |
| A25.3 | 对 CCP 管理器的影响已被承认 | `windows_blocker` 与引擎阶梯对「已这样跑过」的机器不再适用（桌面安装器自己整段跳过）；「安装最新版」从 0.2.5 起拿到的就是本地模式。仍然需要引擎的部分已明确写出：agent-computer 的浏览器、agent-langgraph、supervisor、SPIRE |
| A25.4 | 管理器侧不改契约 | 本次只增文档，不动 `crates/claude-codex-pro-core/src/ccpbot.rs` 的命令面；`cargo test -p claude-codex-pro-core --lib ccpbot` 与 `cargo test -p claude-codex-pro-manager --test windows_subsystem` → 24 passed / 4 ignored、`cargo test -p claude-codex-pro-manager --test windows_subsystem` → 90 passed（两仓现状下复跑确认） |
| A25.5 | 0.2.6：内建 Bot 也本机化 | 本地模式从检出根起 `bun --env-file=.env agent-bot/src/index.ts`，只给 `PORT` 与 `MANAGED_AGENT_TOKEN`，用 `MANAGED_AGENT_AG_UI_URL` 告诉服务端；真机证据 `[bot] The built-in Bot is answering (/health 200).` 与 `logs/local-bot.log` 的 `agent-bot listening on http://127.0.0.1:63659/ag-ui`；发布 v0.2.6 资产 setup.exe 4,767,486 B / msi 6,971,392 B / dmg 9,613,574 B（run 41191041617 / 41191041593） |
| A25.6 | 0.2.7：源码自己拿，装完就能用 | 空文件夹的第一次运行不再要求用户自己找检出，而是复用容器路线同一条 `deployment::fetch` 下载发布版 tarball（新步骤 `checkout`）。真机证据：`cargo test … -- local::tests::live_an_empty_folder --include-ignored` → `[checkout] Downloading CCP-Bot itself (once, then reused)…`、`downloaded "version": "0.2.6"`、1 passed 6.13s；同时把 `agent-bot` 补进 `deployment::ALSO_COPIED`（这条用例第一次跑就断言失败在这里）。发布 v0.2.7 资产 setup.exe 4,768,020 B / msi 6,971,392 B / dmg 9,615,973 B |
| A25.7 | 装出来的是应用，且不再要管理员 | 把 v0.2.6 / v0.2.7 的 release 安装包以 `/S /D=<临时目录>` 装出真实应用后启动：窗口标题 `CCP-Bot`，首屏 `Your own AI coworkers, on this computer.` + `Set up CCP-Bot`，**没有** `This account cannot install Windows components`；点 `Run it on this computer instead` 进入向导 `STEP 1 OF 4`。装出来的 0.2.7 二进制含 `Downloading CCP-Bot itself` 与 `agent-bot/src/index.ts`；注册表键显示 `DisplayVersion = 0.2.7`（A24 的检测看得见）。两个探针安装都已 `uninstall.exe /S` 卸净，主人自己在 `H:\CCP-Bot` 的 0.2.4 安装记录已复原 |
