# 验收标准：托盘快捷菜单

验证对象：`spec/tray-quick-menu.md`

## 通过 / 失败标准

### A. 菜单结构与文案

通过：

- `tray_menu.rs` 中 `Menu::with_items` 传入 6 个菜单项，顺序为
  「显示管理工具」「启动或重启 Codex」「启动或重启 Claude」「Claude 一键汉化」
  「在 GitHub 点个 Star」「退出」。
- 菜单项 id 分别为 `show`、`codex_restart`、`claude_restart`、`claude_zh_patch`、
  `github_star`、`quit`，且 `show` / `quit` 与旧菜单保持一致。
- 对外入口签名仍为 `pub fn setup(app: &tauri::App) -> tauri::Result<()>`。

失败：

- 菜单项数量、顺序或文案不符。
- 改动了 `setup` 的签名，或把菜单构建移出 `tray_menu.rs`。

### B. 左键单击与显示主窗口

通过：

- 托盘构建保留 `show_menu_on_left_click(false)`。
- 左键 `MouseButtonState::Up` 时显示主窗口，且执行 `show` + `unminimize` + `set_focus`。
- `show` 菜单项复用同一显示逻辑。

失败：

- 左键点击改为弹菜单，或显示逻辑缺少任一窗口操作。

### C. 动作确实执行（复用既有实现）

通过：

- `codex_restart` 调用 `commands::restart_claude_codex_pro`。
- `claude_restart` 调用 `commands::open_claude_desktop`。
- `claude_zh_patch` 调用 `commands::install_claude_zh_patch`。
- `github_star` 调用 `commands::open_external_url`，URL 为
  `https://github.com/DamonZS/Claude-Codex-Pro-Tool`。
- 未复制启动、汉化、打开链接的算法，未新增等价实现。

失败：

- 菜单项调用空函数、TODO、错误命令，或复制大段既有逻辑。
- 新增了 `commands.rs` 之外的启动/汉化实现。

### D. 非阻塞与失败处理

通过：

- 菜单事件回调内只做分派，不在回调内直接执行进程枚举/结束、UAC 提权、网络请求。
- Codex / Claude / 汉化 / 打开链接都在后台线程或异步任务中执行。
- 失败时写 `manager.tray.<action>` 诊断日志，包含 status 与 message。
- 菜单回调路径无 `unwrap` / `expect` / `panic!`，失败不弹阻塞对话框。

失败：

- 点击后主线程卡顿或托盘无响应。
- 失败时 panic、弹模态框或静默吞掉且无日志。

### E. 改动边界

通过：

- 只改了 `apps/claude-codex-pro-manager/src-tauri/src/tray_menu.rs`，并新建
  `spec/tray-quick-menu.md`、`acceptance/tray-quick-menu.md`。
- `lib.rs`、`commands.rs`、前端文件、`Cargo.toml`、`docs/`、`build-lead/`、`.dsh-ops/` 未被改动。

失败：

- 出现上述范围外的代码或配置改动（文档、spec、acceptance 除外）。

### F. 编译验证

通过：

- 执行以下命令并返回成功（`Finished` / 退出码 0）：

  ```powershell
  $env:CARGO_TARGET_DIR="D:\Project\Claude-Codex-Pro-Tool\build-lead"
  $env:TMP="D:\Project\Claude-Codex-Pro-Tool\.tmp-build"; $env:TEMP=$env:TMP
  cd D:\Project\Claude-Codex-Pro-Tool
  cargo check -p claude-codex-pro-manager
  ```

- `tray_menu.rs` 无新增 compiler warning（unused import / unreachable pattern 等）。

失败：

- `cargo check` 报错且未修复；或未运行验证就宣称完成。

### G. 真机点击验收（由 Lead 验证）

通过（Lead 在 Windows 真机、本轮接线后执行）：

- 右键托盘图标出现 6 项菜单，顺序与文案正确。
- 左键单击托盘图标显示并聚焦主窗口。
- 逐项点击：Codex 启动/重启、Claude 启动/重启、Claude 一键汉化、打开 GitHub 仓库均产生预期效果。
- 点击「退出」后 CCP 退出，且既有退出清理回调仍执行。
- 任一项失败时，诊断日志出现 `manager.tray.*` 记录，界面无阻塞弹窗。

失败：

- 任一项无反应、菜单顺序错误、左键弹菜单、退出未清理进程。
- 本项不由实现代理执行；实现代理只提交证据供 Lead 复验。

## 必需验证命令

实现代理至少运行：

```powershell
$env:CARGO_TARGET_DIR="D:\Project\Claude-Codex-Pro-Tool\build-lead"
$env:TMP="D:\Project\Claude-Codex-Pro-Tool\.tmp-build"; $env:TEMP=$env:TMP
cd D:\Project\Claude-Codex-Pro-Tool
cargo check -p claude-codex-pro-manager
```

并附源码锚点证据（菜单项列表、命令调用点、日志事件名）。

## 完成证据

- 改动/新建文件清单。
- `cargo check` 的完整结果尾部（成功或失败原因）。
- 菜单项 id 列表与 `setup` 签名。
- 复用函数的 `文件:行号` 清单。
- 对照本验收标准逐项说明通过/未通过（G 项标记为待 Lead 验证）。

## 已知非目标

- 不验证 macOS 托盘行为。
- 不验证菜单项按运行状态动态置灰（不在本 spec 范围）。
- 不验证托盘图标视觉改动（未改动）。