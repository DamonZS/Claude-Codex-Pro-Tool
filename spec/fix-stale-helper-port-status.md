# 修复：后端端口状态陈旧导致误报“后端离线”

## 背景

管理工具总览页把 Codex 后端标记为“后端离线”，但实测本机后端进程健康、正在监听默认端口 57321，`POST /backend/status` 返回 `{"status":"ok",...,"transport":"http-helper"}`。

根因：`~/.claude-codex-pro/latest-status.json` 记录的 `helper_port` 与当前真正在监听的端口不一致。

- helper 端口在每次启动时由 `select_platform_loopback_port(requested)` 选取：默认端口 `57321` 若被占用，则回退到一个随机空闲端口（如 55957）并写入状态文件。
- 写入该随机端口的那个实例后来退出，端口 55957 关闭；随后新的后端实例重新绑定默认 `57321`，但陈旧的 `latest-status.json` 仍指向已关闭的 55957。
- `refresh_launch_port_status`（overview 数据来源）只探测状态文件里记录的 `helper_port`，探测 55957 失败即报告 `helper_port_online=false`，前端因此显示“后端离线”，即便 57321 上有健康后端在应答。

证据（本机实测）：
- `latest-status.json` 记录 `helper_port: 55957`，端口 55957 CLOSED。
- 默认端口 57321 OPEN，`POST /backend/status` → `HTTP 200 {"status":"ok","transport":"http-helper","version":"1.2.9"}`。
- CDP 9230（`debug_port`）OPEN 且 `/json` 含 `webSocketDebuggerUrl`——debug 端口在 Windows 固定不漂移（`select_packaged_codex_debug_port` 在 Windows 直接返回 requested），只有 helper 端口会漂移。

## 目标

**包含：**
- 让 `refresh_launch_port_status` 在“记录的 helper 端口探测失败”时，自愈式回退探测默认 helper 端口（`default_helper_port()` = 57321）。
- 若默认端口上的后端通过 `helper_backend_online` 验证在线，则采用默认端口：更新内存中的 `status.helper_port` 与 `helper_port_online=true`，供 overview 正确显示。
- 把自愈后的状态回写 `latest-status.json`，避免下次仍探测陈旧端口。

**不包含：**
- 不改变 helper 端口的选取/绑定逻辑（`select_platform_loopback_port`、`ensure_detached_helper`）。
- 不改变 debug 端口探测（其不漂移）。
- 不改变 `/backend/status` 探测协议、超时或 body 校验。
- 不新增设置项、不改前端 UI、不改数据存储位置。

## 用户视角

用户打开管理工具总览：只要本机确有健康后端在默认端口应答，总览的 Codex 状态就应显示“后端在线”，而不是因为状态文件端口陈旧而误报离线。“修复后端服务”按钮行为不变。

## 功能要求

- `refresh_launch_port_status(status)`：
  1. 先按现有逻辑探测记录的 `helper_port`。
  2. 若记录端口探测为 false（或 `helper_port` 为 None），再探测 `default_helper_port()`：
     - 若默认端口 `helper_backend_online` 为真且默认端口不同于记录端口，则将 `status.helper_port = Some(default)`、`status.helper_port_online = true`。
  3. debug/frontend 探测逻辑保持不变。
- 自愈仅在“记录端口不在线、默认端口在线”时触发；两者都在线或都离线时行为与现状一致（记录端口在线优先，不覆盖）。
- 当自愈实际改变了 `helper_port` 或在线判定时，将修正后的 `LaunchStatus` 回写 `StatusStore::default()`（best-effort，写失败不影响返回值）。

## 技术约束

- 仅改 `apps/claude-codex-pro-manager/src-tauri/src/commands.rs`。
- 复用既有 `helper_backend_online`、`default_helper_port`、`StatusStore`。
- 探测为阻塞式短超时（既有实现 250–500ms），额外一次默认端口探测仅在记录端口失败时发生，不引入常态额外开销。
- 不得破坏 `windows_subsystem.rs` 里对 `refresh_launch_port_status`、`helper_port_online`、`default_helper_port` 的存在性/文本断言。

## 交付范围

- 代码：`commands.rs` 的 `refresh_launch_port_status` 自愈回退 + best-effort 回写。
- 测试：针对回退判定的纯逻辑单元测试（抽出可测的纯函数：给定“记录端口在线、默认端口在线、记录端口号、默认端口号”，返回最终 `(helper_port, helper_port_online)`）。
- 验证：`cargo build -p claude-codex-pro-manager`、相关定向测试、`windows_subsystem` 契约测试通过。
