# CCP-Bot 集成（阶段 1：内嵌工作台）

## 背景

CCP-Bot（原 OpenBot，已完成中文化与品牌改名）是一个独立部署的 Web 应用栈：`app`（React/Vite，默认 :3010）与 `server`（Hono，默认 :3001）在开发模式下由 `bun run dev` 于宿主机并行启动，生产模式下以 Docker Compose 编排（postgres / server / worker / agent-* / supervisor）。

用户本机的 CCP-Bot 源码位于 `D:\Project\OpenBot`。日常工作在 CCP（Claude Codex Pro Tool）中完成，切到浏览器开另一个标签页看 CCP-Bot 打断工作流：两套窗口、两套滚动位置、两套剪贴板焦点。用户据此提出把 CCP-Bot 整合进 CCP。

整合有三种形态，本规格只覆盖第一种：

1. **内嵌界面**：CCP 内新增「CCP-Bot」页面，用 iframe 载入其 Web UI，并提供启动 / 停止 / 探测。
2. 深链 + 状态卡：只在 CCP 显示状态与跳转链接，不内嵌。
3. 源码级融合：把 CCP-Bot 的页面搬进 CCP 的 React 树。

## 目标

1. CCP 主导航新增「CCP-Bot」路由，页面内以 iframe 内嵌 CCP-Bot Web UI，占满可用高度。
2. 提供状态面板：地址、模式（源码 / Docker）、健康探测结果（`GET {address}/health`）、运行时探测（bun 版本、docker CLI 与 daemon、项目目录、端口占用）。
3. 提供「启动」「停止」「重新检测」「在浏览器打开」操作；源码模式启动 `bun run dev`，Docker 模式启动 `docker compose up -d`。
4. 配置持久化到 `<app state dir>/ccpbot.json`，下次打开沿用。
5. 不可达时页面给出可操作的引导，而不是白屏。

## 非目标

- **不下载、不内置、不分发 CCP-Bot 本体**（不含镜像、不含安装器、不打包源码）。CCP 只是一个控制器与容器壳；CCP-Bot 必须在用户机器上以源码目录或 Docker 编排的形式已经存在。
- 不修改 CCP-Bot 源码；不接管其鉴权、会话与数据库。
- 不做单点登录 / 免登录透传（阶段 3）。
- 不做服务端语言协商（CCP-Bot server 报错文案仍是英文，属独立阶段）。
- 不做自动更新 CCP-Bot。

## 用户视角

1. 打开 CCP，侧栏出现「CCP-Bot」。
2. 首次进入看到状态卡：地址 `http://127.0.0.1:3010`、模式「源码」、探测结果「未运行」。
3. 点「启动」，日志尾部滚动输出，探测转为「可达」，页面内嵌界面刷新为 CCP-Bot 首页。
4. 点「停止」，进程树被结束（源码模式）或 compose 栈被停掉（Docker 模式）。
5. 换地址或换模式后点「保存」，重启 CCP 仍是这套配置。

## 功能要求

### 配置

- 字段：`address`（必填，默认 `http://127.0.0.1:3010`）、`mode`（`source` | `docker`，默认 `source`）、`project_dir`（默认 `D:\Project\OpenBot`）。
- 地址规范化：去首尾空格、去尾部 `/`；只接受 `http://` 或 `https://` 前缀，其他一律报错并保持原值不改。
- 持久化文件：`<app state dir>/ccpbot.json`（与 `app-preferences.json` 同目录）；文件缺失或损坏时回落默认值，损坏文件另存 `.corrupt-<millis>.bak`。

### 健康探测

- 请求 `GET {address}/health`，总超时 3 秒，连接超时 2 秒，绕过环境代理（loopback 目标）。
- `2xx` 且响应体包含 `"ok"` → `reachable = true`。
- 其他情况 `reachable = false`，`detail` 必须区分：连接被拒（服务未启动）、超时、DNS 失败、非 2xx（附状态码）。
- 探测本身不得阻塞 UI 超过约 3.5 秒。

### 运行时探测

- `bun_version`：执行 `bun --version` 的输出（无则 `null`）。
- `docker_cli`：执行 `docker --version` 的输出（无则 `null`）。
- `docker_daemon`：`docker info` 是否成功（CLI 存在但 daemon 未启动时为 `false`）。
- `project_dir_present`：`project_dir` 下同时存在 `package.json` 与 `docker-compose.yml`。
- `env_ready`：`project_dir` 下存在 `.env`（源码模式启动的前置条件，`generate:app-config` 需要它）。
- `app_port_open` / `server_port_open`：对地址主机端口与 `server` 端口（默认 3010 / 3001）做 TCP connect，超时 500ms。

### 启动

- 前置校验，任一不满足则返回失败且不产生进程：
  - 源码模式：`project_dir` 存在且 `env_ready`；`bun_version` 非空。
  - Docker 模式：`docker_cli` 存在且 `docker_daemon` 为真。
- 源码模式：在 `project_dir` 执行 `bun run dev`，stdout/stderr 追加写入 `<app state dir>/ccpbot-dev.log`；记录子进程 pid 到 `<app state dir>/ccpbot-dev.pid`；Windows 下不弹控制台窗口。
- Docker 模式：在 `project_dir` 执行 `docker compose up -d`，输出写入 `<app state dir>/ccpbot-compose.log`。
- 重复启动：若 pid 文件存在且进程仍活，或 `app_port_open` 为真，返回「已在运行」而不是再起一份。

### 停止

- 源码模式：读 pid 文件，Windows 用 `taskkill /PID <pid> /T /F` 结束进程树，删除 pid 文件；pid 文件缺失或进程已消失 → 返回「未在运行」。
- Docker 模式：在 `project_dir` 执行 `docker compose down`。
- 停止后不得留下端口占用检查不到的孤儿进程（`/T` 覆盖 bun 派生的 vite 与 hono 子进程）。

### UI / 交互

- 路由：`ccpbot`，侧栏标签「CCP-Bot」，图标 `Bot`，描述「CCP-Bot 工作台：状态、启动与内嵌界面」，关键词包含 `openbot` / `bot` / `workspace` / `harness`。
- 页面结构自上而下：状态行（可达徽标 + 地址 + 模式 + 目录 + 端口）、操作行（启动 / 停止 / 重新检测 / 保存 / 在浏览器打开 / 刷新内嵌）、内嵌区（`flex: 1`，`border: 0`，`width/height: 100%`）。
- 内嵌 iframe：`allow="clipboard-read; clipboard-write; microphone; camera"`、`referrerPolicy="no-referrer"`、`title="CCP-Bot"`；不可达时 iframe 不渲染，改为占位说明 + 「启动」按钮。
- 探测中显示加载态；错误以现有 `notify` 机制提示（中文）。
- 所有文案中文；不新增第三方依赖。

## 数据与接口要求

新增 Tauri 命令（`CommandResult<T>` 包装，`ok` / `failed` 中文消息）：

| 命令 | 入参 | 返回 |
| --- | --- | --- |
| `ccpbot_load` | 无 | `{ config, probe, runtime, pid, logPath }` |
| `ccpbot_probe` | 无 | `{ probe, runtime, pid }` |
| `ccpbot_save_config` | `{ address, mode, projectDir }` | `{ config }` |
| `ccpbot_start` | 无 | `{ started, pid, message }` |
| `ccpbot_stop` | 无 | `{ stopped, message }` |
| `ccpbot_log_tail` | `{ lines }` | `{ path, text }` |

## 技术约束

- HTTP 探测复用 `crates/claude-codex-pro-core/src/http_client.rs` 的 `proxied_client_for(ua, true)`（loopback 绕过环境代理）。
- 子进程创建统一走 `crate::windows_create_no_window()` 关闭控制台窗口。
- 命令层遵循现有范式：`pub async fn` + `CommandResult<T>` + `ok` / `failed`，注册在 `apps/claude-codex-pro-manager/src-tauri/src/lib.rs` 的 `generate_handler!` 列表中。
- 前端不扩展 `AppActions`（该类型与 `App.tsx` 耦合面过大），新页面组件内直接调用 `invokeCommand`（先例：`apps/claude-codex-pro-manager/src/components/SessionIndexDemo.tsx`）。
- 新增路由必须同时改三处：`apps/claude-codex-pro-manager/src/types.ts` 的 `Route` 联合、`apps/claude-codex-pro-manager/src/lib/routes.ts` 的 `routes` 与 `routeSubtitle`（`Record<Route, string>` 缺键即编译失败）、`apps/claude-codex-pro-manager/src/App.tsx` 的渲染分支。
- 不引入新 npm 依赖，不引入新 Rust 依赖（`reqwest`、`serde` 已在 core 中）。

## 交付范围

- `spec/feature-ccpbot-integration.md`、`acceptance/feature-ccpbot-integration.md`。
- `crates/claude-codex-pro-core/src/ccpbot.rs`（配置、探测、运行时、启动、停止）+ 单元测试。
- `apps/claude-codex-pro-manager/src-tauri/src/commands.rs` 六个命令 + `lib.rs` 注册。
- `apps/claude-codex-pro-manager/src/components/CcpBotScreen.tsx`、`src/lib/ccpbot.ts`。
- `src/types.ts`、`src/lib/routes.ts`、`src/App.tsx` 路由接线。
- 验证证据：`cargo test -p claude-codex-pro-core ccpbot`、`cargo test -p claude-codex-pro-manager --test windows_subsystem`、`npm --prefix apps/claude-codex-pro-manager run check`、`npm --prefix apps/claude-codex-pro-manager run vite:build`。

## 关键风险

1. **混合内容**：Tauri 生产环境页面源为 `https://tauri.localhost`，iframe 指向 `http://127.0.0.1:3010`。按 W3C potentially-trustworthy 规则 loopback 被视为安全源，不会被 mixed-content 拦截；若个别 WebView2 版本仍拦截，回落方案是用 `WebviewWindow` 单独开窗（阶段 1 只记录，不实现）。
2. **iframe 登录态**：CCP-Bot app 走同源 API，iframe 内属独立 browsing context，浏览器存储可用，不做透传即可工作；若其前端依赖 `window.top` 或 pops，需阶段 3 处理。
3. **孤儿进程**：源码模式停止必须按进程树杀（`/T`），否则 vite/hono 子进程残留并占用 3010/3001。
4. **源码模式需要 `.env`**：仓库不带 `.env`，缺失时 `generate:app-config` 直接失败。启动前用 `env_ready` 显式拦截并给出中文提示，避免用户看到一堆无关报错。
## 其他 CCP 用户怎么开始（分发视角）

CCP 用户装的是 CCP，不是 CCP-Bot。所以页面必须能承接三种起点，而不是假定某个目录存在：

1. **本机已经有 CCP-Bot 在跑**：只填地址（`http://127.0.0.1:3010`，或团队里别人机器的 `http://10.x.x.x:3010`），不用填目录、不用点启动，直接看内嵌界面。
2. **本机有 CCP-Bot 源码或仓库**：点「选择目录」选到含 `package.json` 的目录，保存后点启动；源码模式要求该目录有 `.env`。
3. **完全没有**：页面显示引导，告诉他需要先取得 CCP-Bot，或只在别处部署后用地址接入。真正的「一键下载/安装」属阶段 2，本轮不实现。

由此得出一条硬约束：**默认项目目录必须为空**。把开发机上的 `D:\Project\OpenBot` 写死成默认值，只会让每个用户打开页面第一眼就看到一条属于别人的路径。
## 阶段 2（已实现）：「下载并准备」

分发缺口用一条命令关掉：`ccpbot_prepare`。目标目录为空时，它按顺序做四件事，每件都写进返回消息：

1. `git clone --depth 1 https://github.com/DamonZS/CCP-Bot.git .`（目标是 `DEFAULT_REPO_URL`，即**汉化改名后的这一份**，所以“下载 CCP-Bot”和“下载这个页面内嵌的东西”是同一个产物，不会分裂成两个会漂移的产品）；
2. `bun install`（用 `resolve_bun` 找到真 `bun.exe`，不是 PATH 上的 `bun.cmd` shim）；
3. 从 `.env.example` 复制 `.env`（不覆盖已有 `.env`，密钥仍由用户填）；
4. 打印结果与日志路径 `<state>/ccpbot-prepare.log`。

拒绝而不是猜测的三种情况：目录为空字符串（提示先选目录）、目录非空且没有 `package.json`（提示换一个空文件夹或直接选已有目录）、机器上没有 git / 没有 bun（提示装什么）。原因：往别人的目录里克隆，比直接拒绝糟糕得多。

已经存在 `package.json` 的目录视为“已有 CCP-Bot”，跳过下载；已有 `node_modules` 与 `.env` 同样跳过，所以这一步可以重复执行。
## 已发布产物（下载源在哪）

「下载并准备」能成立的前提是远端真的有一份可下载的汉化版。当前发布状态：

| 交付物 | 位置 | 验证方式 |
| --- | --- | --- |
| 源码仓库 | `https://github.com/DamonZS/CCP-Bot.git`（默认分支 `master`） | `git clone --depth 1` 实测，见 A16 的 live 用例 |
| 源码压缩包 | `https://github.com/DamonZS/CCP-Bot/archive/refs/tags/v0.2.0.tar.gz` | HTTP 200；解包后 `package.json` 为 `name: ccpbot / version: 0.2.0`，`app/src/lib/i18n/messages/zh-CN.ts` 为 134239 字节 |
| 版本标签 | `v0.2.0`（与 `package.json` 的 `0.2.0` 对齐，桌面安装器直接把它当 tag 用） | `resolve_version()` 取 `tag_name` 当版本，不再拼接前缀 |
| 发布清单 | `https://github.com/DamonZS/CCP-Bot/releases/download/v0.2.0/container-images.json` | 匿名 HTTP 200；五个键 `server / supervisor / agent-computer / agent-bot / agent-langgraph` 与 `deployment.rs` 的 `IMAGE_VARIABLES` 逐一对应 |
| 容器镜像 | `ghcr.io/damonzs/ccpbot[-agent-computer|-supervisor|-agent-bot|-agent-langgraph]@sha256:…` | 匿名 `HEAD /v2/.../manifests/v0.2.0` → 200；`docker pull …@sha256:c042d958…` 成功 |

镜像由仓库内的 `.github/workflows/publish-images.yml` 在 `v*` 标签上构建发布，不在开发机上发布：`ghcr.io` 的写权限属于仓库（`GITHUB_TOKEN` 的 `packages: write`），不属于某个人的 OAuth token —— 实测 `gho_` token 登录成功但 `docker push` 报 `denied: permission_denied: The token provided does not match expected scopes.`。

同一份清单是「桌面安装器」和「集群 chart」的共同真相：`deployment.rs` 的 `image_variables()` 与 `charts/ccpbot` 的 `image.tag` 都读它，所以镜像漏掉一个名字时，安装器会拒绝而不是跑起一个混合版本的栈。
### 三条安装路径的验证状态

| 路径 | 入口 | 验证 |
| --- | --- | --- |
| 页面内嵌 + 源码启动 | `ccpbot_prepare` → `bun run dev` | live 用例 `live_prepare_downloads_and_seeds_env`（真实克隆 + `bun install`）、`live_start_serves_health_and_stop_frees_the_port`（真起真停 + 端口释放） |
| 桌面安装器 | `deployment::fetch` 读 `container-images.json` | `live_latest_release_is_downloaded_and_pinned`：下载 `v0.2.0`、铺开、清单比对、离线复用 |
| 集群 chart | `charts/ccpbot` + release 清单里的 digest | `helm template` 渲染出 `ghcr.io/damonzs/ccpbot:v0.2.0`（及 `@sha256:` 形式） |