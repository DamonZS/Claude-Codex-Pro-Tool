# 供应商模型映射投影到 Codex 原生模型目录验收标准

对应规格：`spec/codex-native-model-catalog-from-supplier-mapping.md`

## 通过标准

1. Codex 目标 profile 的模型映射非空时，应用该 profile 会在 `<home>/model-catalogs/relay-<id>.json` 生成目录文件，顶层为 `{"models":[...]}`，条目数等于去重后的映射行数。
2. 生成的 `config.toml` 含有根键 `model_catalog_json`，其值为该目录文件的绝对路径。
3. 目录文件条目中 `slug` 等于「实际请求模型」，`display_name` 等于「菜单显示名」，`visibility` 为 `list`，`supported_in_api` 为 `true`。
4. 不勾「1M 上下文」时，目录条目的 `context_window` 与 `max_context_window` 都等于该行「最大上下文」K 值 × 1000（K 为空按默认 `256` → 256000），且 `config.toml` 的 `model_context_window` 等于首行同一数值。
5. 映射为空时 `config.toml` 不出现 `model_catalog_json`，且不生成 `model-catalogs/` 目录。
6. profile 原有 `model_catalog_json`（用户自配）在映射为空时被原样保留。
7. 清除中转配置后 `config.toml` 不再包含 `model_catalog_json`（回归既有行为）。
8. Claude 目标 profile 不写 `model_catalog_json`，也不生成 `model-catalogs/` 目录。
9. 目录文件先于 `config.toml` 落盘：不存在“config.toml 引用了尚未写出的目录文件”的中间态。
10. 映射解析与去重规则与 `crate::model_catalog::relay_profile_catalog_models` 完全一致，未引入第二套解析实现。
11. 未新增或改动 `assets/inject/` 下任何文件，未新增任何 DOM / 状态 / `model/list` 注入路径。
12. 勾「1M 上下文」时，目录条目 `context_window` 与 `max_context_window` 均为 `1000000`，`config.toml` 的 `model_context_window` 为 `1000000`，同行 K 输入框填写的数值不影响结果。
13. 前端「最大上下文」列头为「最大上下文（K）」，输入框带 `K` 后缀、只接受数字、默认 `256`；勾选 1M 时输入框禁用并显示 `1000`，取消勾选后恢复用户原值。
14. 映射/直连模型区块既有死 CSS 已清理：`.supplier-direct-model-list`、`.supplier-direct-model-list-body`、`.supplier-direct-model-list-head`、`.supplier-direct-model-empty`、`.supplier-mapping-card` 在 `apps/claude-codex-pro-manager/src/styles.css` 与 `src/workspace.css` 中无残留；清理后 `tsc` 与 `vite:build` 通过，同区块活跃选择器（`.supplier-codex-catalog-*`、`.supplier-direct-model-section`、`.supplier-direct-fields-*`、`.supplier-direct-toggle-*`、`.supplier-advanced-card`、`.supplier-ccswitch-savebar`）未受影响。

## 必需验证方式

```powershell
cargo fmt --check
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml --test relay_config -- --nocapture
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml relay_config -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
git diff --check
```

端到端证据（无侵入，不改用户 `~/.codex`）：

```powershell
$home2 = Join-Path $env:TEMP "ccp-catalog-e2e"
# 用 CCP core 测试覆盖的应用路径把该 profile 应用到 $home2，再断言：
#  1) Test-Path "$home2\model-catalogs\relay-<id>.json"
#  2) (Get-Content "$home2\config.toml" -Raw) -match 'model_catalog_json'
#  3) codex debug models 在 CODEX_HOME 指向 $home2 时列出映射中的 slug
```

## 必需证据

- `cargo test ... --test relay_config -- --nocapture` 的完整输出，显示新增用例通过。
- 生成的 `config.toml` 片段（含 `model_catalog_json` 行）与目录文件内容片段。
- 若执行了 Codex 端到端验证：`codex debug models` 在临时 `CODEX_HOME` 下的 "total=映射条数" 与 slug 列表。
- 验证结束后临时目录被清理，用户 `~/.codex/config.toml` 与 `~/.claude-codex-pro/settings.json` 未被修改。

## 非目标检查

- 不检查、不要求 Codex 官方安装目录下任何文件被修改。
- 不要求把 Codex 内置模型合并进映射目录（替换语义是本次约定）。
- 不要求验证上游供应商是否真的提供某个模型。
- 不要求改变「应用预设」清空映射或 Codex 目标「获取模型」不写映射表的既有行为。
- 不要求多账号、多操作系统行为差异验证，仅要求 Windows 本机证据。

---

## 执行记录（2026-09-11）

### 1. 判据工具

`C:\Users\Damon\AppData\Local\OpenAI\Codex\bin\7ac07f4ce733f89a\codex.exe`（v0.153.4）提供 `codex debug models`：直接打印渲染引擎**实际加载**的模型目录 JSON，是判定 `model_catalog_json` 是否生效的金标准（比重启客户端目视更可靠，且完全只读）。

### 2. 端到端证据（真实 `~/.codex`）

命令：`codex debug models`

结果：`EXIT=0`，输出恰好 5 条，与供应商映射逐条对应：

| slug | visibility | supported_in_api |
| --- | --- | --- |
| deepseek-v4.1-flash | list | true |
| deepseek-v4-flash-vision-exp | list | true |
| deepseek-v4-pro | list | true |
| glm-5.3 | list | true |
| glm-5.3-flash | list | true |

补充观察：引擎会把条目里的 `shell_type` 规范化为 `unified_exec`；未报任何目录加载错误。

### 3. 最小字段控制实验

用 `catalog_entry_from_mapped_model` 实际产出的字段集（12 字段）生成目录文件，在临时 `CODEX_HOME` 下验证：

`codex debug models -c "model_catalog_json=<临时目录文件>"` → `EXIT=0`、`COUNT=5`，5 条 slug 全部命中且 `visibility=list`。

结论：**代码产出的字段集已足够**，无需为对齐官方 28 字段模板而扩展字段。

### 4. 定向测试

| 命令 | 结果 |
| --- | --- |
| `cargo test -p claude-codex-pro-core --test relay_config` | 109 passed; 0 failed |
| `cargo test -p claude-codex-pro-core --test model_catalog` | 20 passed; 0 failed |

本次新增/改名用例：`apply_relay_profile_maps_codex_catalog_to_native_model_catalog_json`、`apply_relay_profile_mapping_overrides_previous_model_catalog_json`、`apply_relay_profile_sanitizes_catalog_file_name`、`apply_relay_profile_for_claude_target_skips_model_catalog_json`、`clear_relay_config_removes_generated_model_catalog_json`；原 `..._does_not_write_model_catalog_json_for_selected_models` 改名为 `apply_relay_profile_without_mapping_does_not_write_model_catalog_json`。

### 5. 逐条对照

| # | 验收项 | 状态 | 证据 |
| --- | --- | --- | --- |
| 1 | 非空映射生成 `model-catalogs/relay-<id>.json` | 通过 | `apply_relay_profile_maps_codex_catalog_to_native_model_catalog_json`；现场文件已落盘 |
| 2 | `config.toml` 含根键 `model_catalog_json` 指向绝对路径 | 通过 | `config.toml` 第 12 行；`inject_supplier_model_catalog` 用 `canonicalize` 取绝对路径 |
| 3 | `slug`/`display_name`/`visibility=list`/`supported_in_api=true` | 通过 | 第 2 节实测表 |
| 4 | 带上下文字段时出现 `context_window`/`max_context_window` | 通过 | 现场目录文件两条字段齐全（128000）；无该字段的映射不写入（代码分支受 `Option` 控制） |
| 5 | 空映射不写键、不建目录 | 通过 | `apply_relay_profile_without_mapping_does_not_write_model_catalog_json` |
| 6 | 空映射保留用户自配 `model_catalog_json` | 通过 | `apply_relay_profile_without_mapping_preserves_user_model_catalog_json` |
| 7 | 清除中转配置后键被移除 | 通过 | `clear_relay_config_removes_generated_model_catalog_json` |
| 8 | Claude 目标 profile 跳过 | 通过 | `apply_relay_profile_for_claude_target_skips_model_catalog_json` |
| 9 | 目录文件先于 `config.toml` 落盘 | 通过 | `inject_supplier_model_catalog` 中 `std::fs::write` 在 `apply_relay_files_to_home` 之前，写失败即整体失败 |
| 10 | 解析/去重复用 `relay_profile_catalog_models` | 通过 | 未新增第二套解析；该函数仅提升可见性为 `pub(crate)` |
| 11 | 未新增或改动 `assets/inject/` 注入路径 | 通过 | `renderer-inject.js` 内检索 `model_catalog`/`model/list` 零命中；该文件本轮 diff 属独立的「注入增强删除按钮」任务 |

### 6. 现场状态说明

- 运行中的 CCP 进程 PID 7580（12:45:31）与 PID 22760（12:45:21）**早于**修复构建 `target\release\claude-codex-pro.exe`（13:03:52）：这是旧二进制，不会写入 `model_catalog_json`，也是「下拉里看不到映射模型」的直接原因。该二进制字符串检索确认已含 `model_catalog_json`（3 处）与 `model-catalogs`（1 处）。
- 该进程正承载 `127.0.0.1:57321` 代理，按用户要求**不重启**。
- 为让原生下拉立即可用，已手工落盘与代码等价的最小现场产物（见第 7 节）。
- Codex 桌面端需重启（新会话）后重新读取 `config.toml`，原生「选择模型」才会列出这 5 个模型。

### 7. 手工落盘产物与回滚

- 目录文件：`C:\Users\Damon\.codex\model-catalogs\relay-mycodex-1782385192571-ccswitch-copy.json`
- `config.toml` 第 12 行：`model_catalog_json = "C:\\Users\\Damon\\.codex\\model-catalogs\\relay-mycodex-1782385192571-ccswitch-copy.json"`（`tomllib` 解析通过）
- 备份：`C:\Users\Damon\.codex\config.toml.ccp.handoff.20260911-131437.bak`（9,828 字节）
- 回滚：删除该行并删除 `model-catalogs\` 下的该文件，或直接还原备份。

### 8. 已知未覆盖项

- 未重启 Codex 桌面端做人工下拉目视确认（需用户侧操作，属环境限制，非代码问题）。
- 仅 Windows 本机验证，未覆盖多平台。

---

## 执行记录（2026-10-22：最大上下文改 K 单位与 1M 语义）

背景：用户反馈在「模型映射」把最大上下文填成 `256000` 后，`config.toml` 预览仍是 `model_context_window = 1000000`；要求「单位改成 K、默认 256K，不勾 1M 填多少落多少，勾了 1M 就落 1M」。

规则变更：旧「第 4 条」按下述新语义生效（旧语义 = 不勾 1M 时只写 `max_context_window`、`config.toml` 恒为 1000000）。

### 1. 判据命令（2026-10-22 实跑）

| 命令 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | 通过（退出码 0） |
| `cargo test -p claude-codex-pro-core --manifest-path Cargo.toml --test relay_config` | 117 passed；0 failed |
| `cargo test -p claude-codex-pro-core --manifest-path Cargo.toml --test model_catalog` | 20 passed；0 failed |
| `npm --prefix apps/claude-codex-pro-manager run check` | `tsc --noEmit` 退出码 0 |
| `npm --prefix apps/claude-codex-pro-manager run vite:build` | 构建成功（7.99s） |
| `cargo build --release` | 成功；`target/release/claude-codex-pro.exe` 27,950,080 字节 @ 2026-10-22 17:14（含本轮全部改动） |
| `cargo test --workspace --no-fail-fast` | **全绿（退出码 0）**：`--test windows_subsystem` 90 passed/0 failed、`--test request_telemetry` 15/0（无代理与带 7897 代理两种环境）、`--test launcher` 67/0、`--test relay_config` 117/0、`--test model_catalog` 20/0、`--lib` 568/0（16 ignored） |
| 集成期附带修复（各有独立文档） | ① `launcher.rs` 删除被 `eadfe32` 废弃的陈旧单测（用户裁决）；② `--test launcher` 补 8 处 `provider-sync` 期望（既有失败，`FakeHooks` 漏记 `settings.rs:564` 默认值）；③ `--test windows_subsystem` 对齐 12 处既有漂移（`40da83e`/`e05ff36`/`ac1d81b`/`0e697e7`，产品代码零改动）；④ `request_telemetry` 既有失败由环回代理绕过修复，见 `spec/relay-loopback-ambient-proxy-bypass.md` |

### 2. 逐条对照

| # | 验收项 | 状态 | 证据 |
| --- | --- | --- | --- |
| 4 | 不勾 1M 时 `context_window` = `max_context_window` = K×1000 | 通过 | `apply_relay_profile_respects_disabled_codex_one_m_context`（256000/256000）；`supplierCodexCatalogJson` 不勾分支写 `supports1m:false` + `maxContextWindow` |
| 4 | 不勾 1M 时 `config.toml` 的 `model_context_window` 同步 | 通过 | 同一用例断言 `model_context_window = 256000`；`relay_profile_context_window` 改用 `codex_catalog_effective_context_window` |
| 12 | 勾 1M 时条目与 `config.toml` 均为 1000000，忽略更小的 K 值 | 通过 | `apply_relay_profile_codex_one_m_ignores_smaller_max_context`（`supports1m:true` + `maxContextWindow:256000` → 1000000/1000000） |
| 13 | 前端 K 输入框（列头、（K）后缀、默认 256、勾 1M 禁用显示 1000） | 通过 | `SupplierScreen.tsx` 映射行改用 `.supplier-codex-catalog-limit` 包装；`styles.css` 新增该规则；`npm run check` 与 `vite:build` 通过 |
| 14 | 映射/直连区块既有死 CSS 清理 | 通过 | `grep -rn` 上述 5 个选择器在 `apps/claude-codex-pro-manager/src` 的 `.css` 中只命中 `styles.css.bak`（预先存在的备份文件，不属清理范围）；`styles.css` 5 处、`workspace.css` 5 处删除；删除后 `npm run check`=0、`vite:build` ✓ 3.72s、`cargo test --workspace --no-fail-fast` 仍全绿 |

### 3. 已知取舍

- `config.toml` 只有一个 `model_context_window`，多行映射时沿用既有「取首行」约定。
- 历史行（无 `supports1m` 字段）保持旧的 `min()` 语义，避免影响既有 cc-switch 导入行。
- 前端判定纯函数 `supplierCodexEffectiveContextWindow` 与后端 `codex_catalog_effective_context_window` 同规则双实现，用于让弹窗预览与实际落盘一致。
