# Claude 一键汉化：提权卡死 + locale 校验失败修复

## 背景

用户点击「Claude 一键汉化」（概览页命令条主按钮）后，界面长时间卡在 running 态提示「正在请求管理员授权并写入 Claude 本机汉化资源」，最终汉化没成功、Claude Desktop 也没重启。用户明确反馈：**以前不需要管理员授权就能写入**。

经日志（`~/.claude-codex-pro/claude-codex-pro.log`）、磁盘状态、备份目录三方硬证据定位，这是**两个独立缺陷叠加**，都发生在 MSIX 版 Claude（`installKind == "msix"`，安装在 `C:\Program Files\WindowsApps\Claude_*`）的提权补丁路径上。「以前不用授权」是因为当时可能是可写的桌面版；MSIX 版目录受系统保护不可写，`patch_needs_elevation()` 返回 true 而每次要 UAC —— 这条判定本身正确，不是缺陷；缺陷在提权之后。

### 缺陷 #1：提权子进程跑完不退出，父进程 `-Wait` 挂到超时（“一直在跑”的直接原因）

- 提权流程：`install_claude_zh_patch()`（`apps/claude-codex-pro-manager/src-tauri/src/commands.rs`）检测到需要提权后，调 `install_claude_zh_patch_elevated()` → `run_claude_zh_patch_elevated()`，用 PowerShell `Start-Process -Verb RunAs -Wait -PassThru` 拉起**自身 exe** 带 `--internal-install-claude-zh-patch` 参数的提权子进程，父进程同步等待其退出（`run_elevated_process_with_timeout`，超时 `CLAUDE_ZH_PATCH_ELEVATED_TIMEOUT`，实测约 5 分钟）。
- 提权子进程入口是 `main()`（`apps/claude-codex-pro-manager/src-tauri/src/main.rs`）：
  ```rust
  fn main() {
      if claude_codex_pro_manager_lib::commands::handle_internal_cli() {
          return;
      }
      // 否则继续启动完整 Tauri GUI
      ...
  }
  ```
- `handle_internal_cli()`（`commands.rs`）执行内部安装/还原后，**返回 `cli_result.status == "ok"`（bool）**：补丁成功返回 `true`（main 早退），补丁失败返回 `false`。
- 缺陷：当补丁判定失败（见缺陷 #2），`handle_internal_cli()` 返回 `false` → `main()` 不早退 → 提权子进程**继续启动成一个完整的、以管理员身份运行的 GUI，永不退出**。父进程 `Start-Process -Wait` 死等它退出，一直挂到 5 分钟超时才 bail，全程 UI 卡在 running 态。
- 日志铁证：提权子进程 `pid 3032` 打完 `manager.claude_zh_patch.internal.finish`（status=failed）后仅 2ms 就打出 `manager.start`，随后加载概览/设置/插件仓库 —— 它变成了第二个常驻 GUI。父进程只有 `elevated.start`，无 `elevated.exit`（在超时前被观察）。

### 缺陷 #2：locale 配置在长时间补丁过程中被重启的 Claude 覆盖回 en-US，导致完整性校验永远失败（“汉化没成功”的直接原因）

- 补丁完整性由 `status_for_paths()`（`crates/claude-codex-pro-core/src/claude_zh_patch.rs`）判定，需要 6 项全 true 才 `status == "ok"`：`resources_present / frontend_i18n_present / statsig_i18n_present / locale_configured / chunk_patch_present / language_whitelist_patched`。
- 实测结果：`resources=true frontend=true statsig=true locale=false chunk=true language=true` —— 唯一失败项是 `locale`。
- `locale_configured()` 读 `paths.locale_config_path`（MSIX 下经 `with_user_data_dirs` 解析为 `%LOCALAPPDATA%\Claude-3p\config.json`）并要求其中 `"locale" == "zh-CN"`。
- 磁盘/时间戳/备份证据：
  - 备份目录 `%LOCALAPPDATA%\Claude-zh-CN-official-backup\` 里存在 `C____Users__Damon__AppData__Local__Claude-3p__config.json` 的备份 —— 证明补丁**确实瞄准了正确的 Local 路径**，写入路径解析和 `atomic_write` 都没错。
  - 但当前该 Local `config.json` 的 `"locale"` 是 `"en-US"`，其 mtime（16:56:40）落在补丁运行窗口内（internal.start 16:55:32 → finish 16:58:01）。
  - `install_patch_at_with_resources_impl()` 的写入顺序：先写 i18n 资源 → **第 4 步 `write_locale_config()` 写 `locale=zh-CN`** → 之后 `find_patchable_chunks()` + 逐个 `patch_chunk()`（chunk 数量多，实测耗时约 2.5 分钟）→ 最后才 `status_for_paths()` 校验。
- 根因：`install_claude_zh_patch_internal()` 开头调了 `close_claude_desktop_for_patch()` 关闭 Claude，但 **MSIX 版 Claude 会被系统/用户自动重新拉起**；在 locale 写入（早）到最终校验（晚约 2.5 分钟）的长窗口里，重启的 Claude 把它自己的 `config.json` 的 `locale` 覆盖回 `en-US`。最终 `status_for_paths()` 读到 en-US → `locale_configured=false` → 整体判 failed。这又触发缺陷 #1 的 `false` 返回，形成“卡死 + 没成功”的合并症状。

## 目标

### 包含

1. **修缺陷 #1**：确保提权/内部 CLI 子进程在完成 `--internal-install-claude-zh-patch` / `--internal-restore-claude-zh-patch` 后，**无论成功失败都立即干净退出，绝不 fall through 成 GUI**。父进程 `Start-Process -Wait` 应能及时拿到退出码，不再挂到超时。
2. **修缺陷 #2**：确保补丁完成后 `locale_config` 稳定为 `zh-CN`，不被补丁过程中重启的 Claude 覆盖，从而 `locale_configured` 校验能通过、整体 `status == "ok"`。
3. 保持既有安全边界与行为：提权判定逻辑（`patch_needs_elevation` / MSIX 需 UAC）不变；备份/还原语义不变；不弱化完整性校验（不得靠“跳过 locale 检查”来蒙混通过）。

### 不包含

- 不改提权是否必要的判定（`detected_patch_needs_elevation` / `install_root_patch_needs_elevation` / `patch_needs_elevation`）—— MSIX 需要 UAC 是正确行为。
- 不改任何用户可见文案的语言（不做汉化/翻译；遵守 AGENTS.md 安全边界）。仅当为表达新行为必须新增/调整一两条提示时，可加中文提示，但不得批量改写既有文案。
- 不改前端 `installClaudeZhPatch` 的交互流程与按钮（后端返回结构不变时前端无需改）。
- 不改 i18n 资源内容、chunk patch 算法、语言白名单逻辑。
- 不删除用户数据、备份，不改动 Claude 官方文件的补丁/还原策略边界。

## 用户视角

用户在概览页点击「Claude 一键汉化」：
- 弹出 UAC，授权后补丁在后台提权执行；**几十秒内**（而非卡到 5 分钟超时）返回明确结果。
- 补丁成功后：状态显示已完整安装（6 项校验全绿含 locale），并按既有逻辑自动重启 Claude Desktop，界面变中文。
- 若用户取消 UAC 或补丁真失败：及时返回可读的失败原因，不再无限卡在 running 态。

## 功能要求

1. **内部 CLI 子进程退出（缺陷 #1）**：
   - 提权子进程执行内部安装/还原命令后，必须显式终止进程（例如在 `handle_internal_cli` 完成写结果文件后 `std::process::exit(code)`，或在 `main` 中对内部命令分支无条件 `return`/`exit`，不依赖 `handle_internal_cli` 的 bool 是否为 true）。
   - 退出码需能区分成功/失败，供父进程 `run_elevated_process_with_timeout` / 上层判定使用（成功 0，失败非 0）。
   - 必须先把结果 JSON 写入 `result_path`（父进程据此读取 `ClaudeZhPatchCliResult`），再退出；不得因提前退出丢失结果文件。
   - 不得影响正常（非内部命令）启动路径：无 `--internal-*` 参数时仍正常启动 GUI。
2. **locale 配置稳定为 zh-CN（缺陷 #2）**：
   - 采用能消除“补丁过程中 Claude 重启覆盖 locale”竞态的方案。可选实现（择一或组合，由实现代理判断最小改动）：
     - 将 `write_locale_config()` 调整为在**所有 chunk 补丁完成之后、`status_for_paths()` 校验之前**的最后一步写入（缩短被覆盖窗口至最小）；并/或在校验前对 locale 做一次“确认为 zh-CN，否则重写”的兜底。
     - 和/或在补丁全程更强力地阻止 Claude 运行（如打补丁期间循环确认 Claude 已关闭；但不得引入无限等待，需有上限与超时）。
   - 不得通过弱化/删除 `locale_configured` 校验来“修复”——校验语义必须保留，最终磁盘上的 Local `config.json` 的 `locale` 必须真实为 `zh-CN`。
   - 需兼容 `config.json` 已存在其它键的情况（保留其它键，仅设 `locale`，沿用现有 `write_locale_config` 的 merge 行为）。
3. **完整性判定不回归**：`status_for_paths` 的 6 项校验维持不变；修复后在真实 MSIX 环境应达成 6 项全 true。
4. **日志可观测**：保留/补充关键事件（elevated.start/exit、internal.start/finish、locale 重写兜底等），便于验证退出与 locale 落盘。
5. **发布级健壮性（缺陷 #2 加固，面向 GitHub 公开分发的多样环境）**：本工具会发布到 GitHub 供大量用户自行安装，运行环境不可控（不同 Claude 版本、MSIX/桌面版混装、开机自启、有看门狗自动拉起、杀软干扰）。因此 locale 竞态必须做到「在实际可达范围内消除」，而不仅是「缩小窗口」：
   - **紧邻关闭**：在写 locale 的**紧邻之前**（chunk 全部完成之后）再执行一次 `close_claude_desktop_for_patch()`（或等效的「确认 Claude 已退出」逻辑）。因为初次关闭到此刻已隔了约 2.5 分钟的 chunk 打补丁耗时，其间 MSIX/自启环境的 Claude 极可能已被系统重新拉起并在内存中持有 en-US，会在退出时把 `config.json` 刷回。必须在 locale 落盘前把这个「已复活的 Claude」再关掉。
   - **落盘顺序**：locale 写入必须是「关闭 Claude → 写 locale（含兜底确认 zh-CN）→ 校验」这一序列里的最后写操作，且此序列要尽量紧凑，使「locale 落盘」到「后续 `complete_claude_zh_patch_install` 主动重启 Claude」之间不再有其它耗时步骤。
   - **重启后不被回刷的说明**：`complete_claude_zh_patch_install` 成功后会主动重启 Claude；实现须保证被重启的是「读取到 zh-CN 后的新实例」，而非补丁期间残留的旧 en-US 实例（例如确保重启前旧实例确已终止）。
   - **有界等待**：所有「确认 Claude 已关闭」的等待必须有超时上限（参考既有 `wait_for_claude_process_exit` 的 5s 语义），不得引入无限循环或无上限重试导致新的卡死。
   - 不得为此扩大提权范围、不得改写入目标目录策略、不得弱化完整性校验。

## 技术约束

- 改动区域：`apps/claude-codex-pro-manager/src-tauri/src/commands.rs`（`handle_internal_cli` / 提权流程）、`apps/claude-codex-pro-manager/src-tauri/src/main.rs`（内部命令早退）、`crates/claude-codex-pro-core/src/claude_zh_patch.rs`（locale 写入时机/兜底、必要时关闭 Claude 的稳健性）。
- 安全敏感区（AGENTS.md 已标注 `install_patch_at*` / `run_claude_zh_patch_elevated` 涉及提权写文件）——最小必要改动，不扩大提权范围，不改写入目标目录策略。
- 遵守 [[windows-subsystem-test-contract]]：`apps/claude-codex-pro-manager/src-tauri/tests/windows_subsystem.rs` 对 `commands.rs` / `main.rs` / `claude_zh_patch.rs` 有大量按源码文本的存在性/切片断言（如 `handle_internal_cli()`、`--internal-install-claude-zh-patch`、`detected_patch_needs_elevation()`、`ensure_patch_writable(paths)?;` 等）。若改动撞断言，可同步更新锚点，但不得改动被断言的用户可见文案文本本身。
- 遵守 [[tauri-sync-command-blocks-ui]]：`install_claude_zh_patch` 是 async 命令，内部同步阻塞的提权等待应保持在 spawn_blocking / 不阻塞主线程的既有结构（若已如此则不动）。
- 不引入新依赖。不改数据库、发布脚本、安装流程。

## 交付范围

- 代码：上述三个文件的最小必要改动。
- 测试：新增/更新 `windows_subsystem.rs`（或 core 单测）覆盖：内部 CLI 命令后进程会退出（结构断言：内部分支存在 `process::exit` / 无条件早退）；locale 写入在 chunk 之后或有兜底重写（结构/单测断言）。既有 52 个 windows_subsystem 测试与 core `claude_zh_patch` 单测不回归。
- 文档：本 spec 与对应 `acceptance/claude-zh-patch-elevated-hang-and-locale-fix.md`。
- 不交付：文案汉化、提权判定改动、前端交互改动（除非后端返回结构变化必须联动）。

## 追加约束：新版 Claude chunk 兼容

- 安装补丁时不得对所有 `.js` chunk 无差别写入标记；只有确实存在可替换文本、locale 数组、旧补丁残留或已安装标记的 chunk 才允许写入。
- 对新版 Claude 中不存在可补 locale 白名单数组的 UI chunk，若文本补丁标记、三类 i18n 资源和 locale 配置均满足，则 language whitelist 项视为“不适用但满足”；不得因此把状态误判为 `not_installed`。
- JS 校验失败时必须保留 `node --check` 的 stderr 摘要，不能只返回泛化的“JS 校验失败”。

## 追加约束：vendor/react 依赖 chunk 不参与 UI 汉化补丁

- `vendor-*`、`vendor-react-*` 等第三方依赖或运行时依赖 chunk 不属于 Claude UI 文案补丁目标。
- 即使这些 chunk 中包含 `Settings`、React 错误提示、Suspense 文案等普通英文字符串，也不得因为命中文案关键词而被写入 `TEXT_MARKER`、进入 `changed_files` 或触发 `node --check` 校验失败。
- 状态检测也必须忽略这类 vendor/runtime 依赖 chunk 中既有的文本 marker，避免旧残留影响本机中文补丁的真实安装状态判断。
- 例外：如果 vendor/runtime chunk 存在旧版 unsafe window runtime patch 残留，可进入清理路径；但不得把普通 vendor chunk 当作 UI 文案 chunk 批量改写。
