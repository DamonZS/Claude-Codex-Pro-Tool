# 托盘快捷菜单（cc-switch 风格）

## 背景

当前 CCP 管理工具的托盘菜单只有「显示管理工具 / 退出」两项，实现在
`apps/claude-codex-pro-manager/src-tauri/src/lib.rs` 的 `setup_tray`。
用户日常最常用的三个动作——启动/重启 Codex、启动/重启 Claude、Claude 一键汉化——
必须先打开管理窗口、切到对应页面再点击，托盘没有直达入口。

用户在其它同类工具（cc-switch）中已经习惯「右键托盘 → 一键启动/重启/汉化」的快捷路径。
本任务把托盘菜单升级为 cc-switch 风格的 6 项快捷菜单，让用户不打开主窗口也能完成日常操作。

## 目标

本次工作包含：

- 托盘右键菜单固定为 6 项：显示管理工具 / 启动或重启 Codex / 启动或重启 Claude /
  Claude 一键汉化 / 在 GitHub 点个 Star / 退出。
- 每个菜单项点击后真正执行对应动作，而不是只做占位。
- 动作执行不阻塞托盘事件与主线程。
- 动作失败只写诊断日志，不 panic、不弹阻塞式对话框。

本次工作不包含（非目标）：

- 不新增托盘设置界面、不改托盘图标、不做动态菜单（按运行状态禁用/置灰菜单项）。
- 不改动主窗口 UI、前端任何文件、`lib.rs`、`commands.rs`、`Cargo.toml`。
- 不新增 crate 依赖，不引入新的启动/汉化/打开链接算法。
- 不做多语言托盘文案，不改变窗口关闭生命周期（见 `spec/manager-close-process-lifecycle.md`）。
- 不做 macOS 实机托盘行为验证（本机为 Windows；跨平台行为沿用既有 `open_url` 实现）。

## 用户视角与交互

1. 用户右键点击任务栏托盘图标，看到 6 项菜单，顺序固定。
2. 点击「显示管理工具」：主窗口显示、取消最小化并聚焦。
3. 点击「启动或重启 Codex」：若 Codex 已在运行则重启，否则启动；菜单立即收起，动作在后台进行。
4. 点击「启动或重启 Claude」：复用既有 Claude 启动链路，运行中则重启，未运行则启动。
5. 点击「Claude 一键汉化」：复用既有汉化补丁安装链路（含提权分支）。
6. 点击「在 GitHub 点个 Star」：系统浏览器打开仓库主页。
7. 点击「退出」：退出 CCP，沿用现有退出清理回调。
8. 左键单击托盘图标：仍显示主窗口（不弹菜单）。
9. 任一动作失败：托盘不弹模态框，仅在诊断日志中留下一条记录，用户可继续操作。

## 功能要求

1. **菜单结构固定**：菜单项与其 id 一一对应，顺序为
   `show`（显示管理工具）、`codex_restart`（启动或重启 Codex）、
   `claude_restart`（启动或重启 Claude）、`claude_zh_patch`（Claude 一键汉化）、
   `github_star`（在 GitHub 点个 Star）、`quit`（退出）。`show` 与 `quit` 沿用既有 id。
2. **左键行为不变**：托盘构建必须保留 `show_menu_on_left_click(false)`；左键 `Up` 事件显示主窗口。
3. **显示主窗口**：显示、取消最小化、聚焦 `main` 窗口；窗口不存在时静默忽略。
4. **Codex 启动/重启复用**：调用 `commands::restart_claude_codex_pro`，请求参数使用
   `LaunchRequest` 的 serde 默认值（`app_path` 空、默认 debug/helper 端口、不做 provider sync 跳过），
   由既有 `normalize_launch_request` 自行解析本机 Codex 路径。
5. **Claude 启动/重启复用**：调用 `commands::open_claude_desktop`，其内部
   `claude_codex_pro_core::claude_desktop::open_claude_desktop` 已按「进程中存在 Claude 即重启」实现。
6. **Claude 一键汉化复用**：调用 `commands::install_claude_zh_patch`，不复制补丁安装、提权或回滚逻辑。
7. **打开仓库复用**：调用 `commands::open_external_url` 打开
   `https://github.com/DamonZS/Claude-Codex-Pro-Tool`，沿用其 http/https 校验与跨平台打开实现。
8. **动作不阻塞**：菜单事件回调只做分派；Codex/Claude/汉化/打开链接均在独立后台线程中执行，
   通过 `tauri::async_runtime::block_on` 驱动既有 async 命令；打开链接在后台线程直接调用同步命令。
9. **失败只记日志**：结果 `status == "failed"` 时写
   `claude_codex_pro_core::diagnostic_log::append_diagnostic_log("manager.tray.<action>", {...})`，
   `action` 取 `codex_restart` / `claude_restart` / `claude_zh_patch` / `github_star`；
   日志包含 `status` 与 `message`，不写凭据，不弹对话框。
10. **不 panic**：菜单回调与后台线程内禁止 `unwrap`/`expect`/`panic!`；构造 `LaunchRequest` 失败也走日志分支。
11. **创建失败可诊断**：`tray_menu::setup` 返回 `tauri::Result<()>`，构建失败由既有 `lib.rs` 逻辑记录
    `manager.tray_failed` 并继续运行（主窗口不受影响）。
12. **文案一致**：菜单文案为简体中文，与现有「显示管理工具 / 退出」风格一致，使用「或」而非「/」。

## 数据与接口

- 对外唯一入口（保持不变）：

  ```rust
  pub fn setup(app: &tauri::App) -> tauri::Result<()>
  ```

- 复用的既有命令接口（均为 `pub`，无需新增桥接层）：

  | 动作 | 调用 |
  | --- | --- |
  | Codex 启动/重启 | `commands::restart_claude_codex_pro(LaunchRequest)` |
  | Claude 启动/重启 | `commands::open_claude_desktop()` |
  | Claude 一键汉化 | `commands::install_claude_zh_patch()` |
  | 打开仓库 | `commands::open_external_url(String)` |

- 诊断日志事件名：`manager.tray.codex_restart`、`manager.tray.claude_restart`、
  `manager.tray.claude_zh_patch`、`manager.tray.github_star`；
  线程创建失败与参数构造失败复用同一事件名。

## 技术约束

- 只允许改动 `apps/claude-codex-pro-manager/src-tauri/src/tray_menu.rs`，并新建
  `spec/tray-quick-menu.md`、`acceptance/tray-quick-menu.md`。
  明确禁止改动 `src-tauri/src/lib.rs`、`commands.rs`、前端、`Cargo.toml`、`docs/`、`build-lead/`、`.dsh-ops/`。
- 不新增依赖：`tauri`（含 `tray-icon` feature）、`serde_json`、`claude-codex-pro-core`、`std::thread` 均已可用。
- 只能调用 `commands` 中 `pub` 的接口；不得修改 `commands.rs` 提升私有函数可见性。
- 托盘菜单事件回调运行在 Tauri 主线程，任何可能耗时数秒的动作（进程枚举/结束、UAC 提权、网络）必须离开该线程。
- 不得改变现有退出行为：`quit` 仍调用 `app.exit(0)`，由 `RunEvent::Exit` 执行既有清理。
- Windows 为当前验证平台；`open_external_url` 在非 Windows 走既有 `open` 分支，行为不变。

## 交付范围

- 代码：`tray_menu.rs` 完整实现（6 项菜单 + 左键行为 + 4 个后台动作 + 失败日志）。
- 文档：本 spec 与 `acceptance/tray-quick-menu.md`。
- 验证：`cargo check -p claude-codex-pro-manager` 通过；真机点击验收由 Lead 执行。
- 接线（不由本任务执行）：`lib.rs` 中把 `setup_tray(app)` 调用替换为 `tray_menu::setup(app)`，
  并清理原 `setup_tray` / `show_main_window` 及其独占 import，避免 dead_code 警告。

## 变更记录

| 日期 | 变更 | 说明 |
| --- | --- | --- |
| 2026-11-06 | 初版 | 定义托盘快捷菜单 6 项、动作复用、非阻塞与日志约束 |