# 验收标准：Claude 一键汉化提权卡死 + locale 校验失败修复

验证对象 spec：`spec/claude-zh-patch-elevated-hang-and-locale-fix.md`

## 通过 / 失败标准

### A. 缺陷 #1 —— 提权/内部 CLI 子进程完成后必定退出

**通过：**
- `apps/claude-codex-pro-manager/src-tauri/src/main.rs` 或 `handle_internal_cli()` 中，对 `--internal-install-claude-zh-patch` / `--internal-restore-claude-zh-patch` 命令，执行完成（且写完 result JSON）后**无条件终止进程**（`std::process::exit(code)` 或 main 对内部命令分支无条件 return，不再 fall through 到 GUI 构建）。
- 退出码区分成功（0）/失败（非 0），且**失败时也退出**（不再因返回 `false` 而继续启动 GUI）。
- 无 `--internal-*` 参数的正常启动路径不受影响。
- 结果 JSON 在退出前已写入 `result_path`。

**失败：**
- 补丁失败时进程仍会 fall through 成 GUI。
- result JSON 未写就退出，父进程读不到结果。
- 影响了正常 GUI 启动。

### B. 缺陷 #2 —— locale 稳定为 zh-CN，校验通过

**通过：**
- 采取措施消除“补丁过程中 Claude 重启覆盖 locale”竞态：`write_locale_config` 挪到所有 chunk 补丁之后 / 校验之前的最后写入步骤，**和/或**在 `status_for_paths` 校验前有“locale 非 zh-CN 则重写”的兜底。
- `locale_configured` 校验逻辑**未被删除或弱化**；最终 Local `config.json` 的 `locale` 真实为 `zh-CN`。
- 保留 `config.json` 其它键（merge 行为不变）。

**失败：**
- 通过跳过/注释掉 locale 检查来让 status ok。
- locale 写入仍在 chunk 之前且无兜底（竞态未消除）。
- 覆盖或清空了 config.json 其它键。

### B2. 缺陷 #2 加固 —— 发布级健壮性（面向 GitHub 公开分发）

**通过：**
- **紧邻关闭**：写 locale 的紧邻之前（chunk 全部完成之后）再调用一次 `close_claude_desktop_for_patch()`（或等效「确认 Claude 已退出」逻辑），把初次关闭后约 2.5 分钟内可能被系统/自启/看门狗重新拉起的 Claude 再次关闭，防止其在退出时把 `config.json` 刷回 en-US。
- **落盘顺序**：locale 写入是「关闭 Claude → 写 locale（含兜底确认 zh-CN）→ 校验」序列的最后写操作；该序列到 `complete_claude_zh_patch_install` 主动重启 Claude 之间无其它耗时步骤。
- **有界等待**：所有「确认 Claude 已关闭」的等待均有超时上限（参考既有 `wait_for_claude_process_exit` 的 5s 语义），无无限循环 / 无上限重试。
- 未为此扩大提权范围、未改写入目标目录策略、未弱化完整性校验。

**失败：**
- 写 locale 前未再次关闭 / 确认 Claude 已退出，残留竞态仍在（单机偶发成功但多样环境会出现「状态成功但界面仍英文」）。
- 引入无上限的关闭等待 / 重试，造成新的卡死风险。
- locale 落盘与主动重启之间仍夹着 chunk 打补丁等耗时步骤。

### C. 不回归 / 不越界

**通过：**
- 提权判定（`patch_needs_elevation` / `detected_patch_needs_elevation` / `install_root_patch_needs_elevation`）逻辑未改。
- 无任何用户可见文案被汉化/翻译改写（除非为表达新行为新增极少量中文提示）。
- 未引入新依赖；未改发布/安装/数据库。
- 未回滚工作区中其它会话的改动。

## 必需的验证方式

实现代理**不跑**构建/测试（中转不稳定，验证由评审代理执行）。评审代理必须执行并贴出结果：

1. **类型/编译**：
   - `cargo build -p claude-codex-pro-manager`（或在独立 target 目录，因运行中的 exe 会锁定 `target/debug/claude-codex-pro-manager.exe`）—— 必须通过。
2. **Rust 单测（core）**：
   - `cargo test -p claude-codex-pro-core claude_zh_patch`（独立 target 目录）—— 全绿，尤其 `install_patch_writes_resources_locale_and_safe_chunk_markers`、`elevated_install_skips_preflight_writable_probe`、locale 相关单测不回归。
3. **前端契约测试**：
   - `cargo test -p claude-codex-pro-manager --test windows_subsystem`（独立 target 目录）—— 52+ 全绿；若锚点更新，确认更新的是结构锚点而非文案文本。
4. **静态审查**（评审代理逐条核对）：
   - main.rs / handle_internal_cli 内部命令分支确有无条件 `process::exit`。
   - write_locale_config 调用点在 chunk 循环之后，或存在校验前兜底重写。
   - locale_configured / status_for_paths 6 项校验未被弱化。
5. **真机验证（用户侧，尽力而为）**：
   - 重新构建启动管理器 → 点「Claude 一键汉化」→ UAC 授权 → 观察：数十秒内返回；概览页显示 6 项校验全绿（含 locale）；Claude 自动重启且界面为中文。
   - 日志出现 `elevated.exit`（父进程拿到退出码）、`internal.finish` status=ok、Local config.json 的 locale=zh-CN 落盘。
   - 说明：真机验证依赖用户实际点击 + UAC，评审代理若无法触发，需明确标注“待用户真机确认”，并以 1–4 的自动化证据为主。

## 完成所需证据

- 三类 cargo 命令的实际输出（通过/失败 + 关键行）。
- 静态审查 4 项的代码位置引用（文件:行）。
- git diff 摘要：仅 commands.rs / main.rs / claude_zh_patch.rs（+ 必要的测试文件），无越界改动。

## 非目标 / 不在本次检查范围

- 不验证提权判定是否“该不该弹 UAC”（MSIX 弹 UAC 是既定正确行为）。
- 不做全量汉化完整性人工逐条核对（交由既有 status 校验）。
- 不验证非 MSIX（可写桌面版）路径的行为变化（本次聚焦 MSIX 提权路径的两个缺陷）。

### D. 新版 Claude chunk 兼容

**通过：**
- 安装循环会先判断 chunk 是否确实需要或已有补丁痕迹；无关 `.js` 文件保持原样，不会被写成只有补丁 marker。
- 当新版 UI chunk 没有可补 locale 白名单数组，但已经有文本补丁 marker，且 i18n 资源与 locale 均满足时，`language_whitelist_patched` 为 true，整体状态不误判为 `not_installed`。
- `node --check` 失败返回信息包含 stderr 摘要，便于定位具体语法错误。

**失败：**
- 任意无关 `.js` chunk 被写入 marker 或内容被清空。
- 新版无 locale 白名单数组的已补丁 chunk 仍导致状态 `not_installed`。
- JS 校验失败仍吞掉 stderr。

### E. vendor/react 依赖 chunk 排除

**通过：**
- 构造 `vendor-react-*.js`，其中包含会误触发的普通英文字符串（例如 `Settings`）时，执行 `install_patch_at` 后文件内容保持原样。
- `vendor-react-*.js` 不出现在 `changed_files` 中，不被写入 `TEXT_MARKER`，也不会触发 `node --check` 失败导致整次一键汉化回滚。
- `status_for_paths` 统计 UI chunk 时忽略 vendor/runtime 依赖 chunk 的普通文本 marker 残留。

**失败：**
- vendor/react 依赖 chunk 因普通英文字符串被当作 UI 文案 chunk 改写。
- vendor/react 依赖 chunk 的 JS 校验失败导致 Claude 一键汉化整体失败或回滚。
