# 供应商模型映射投影到 Codex 原生模型目录验收标准

对应规格：`spec/codex-native-model-catalog-from-supplier-mapping.md`

## 通过标准

1. Codex 目标 profile 的模型映射非空时，应用该 profile 会在 `<home>/model-catalogs/relay-<id>.json` 生成目录文件，顶层为 `{"models":[...]}`，条目数等于去重后的映射行数。
2. 生成的 `config.toml` 含有根键 `model_catalog_json`，其值为该目录文件的绝对路径。
3. 目录文件条目中 `slug` 等于「实际请求模型」，`display_name` 等于「菜单显示名」，`visibility` 为 `list`，`supported_in_api` 为 `true`。
4. 映射中带上下文窗口时条目含 `context_window` 与 `max_context_window`；不带时不出现这两个字段。
5. 映射为空时 `config.toml` 不出现 `model_catalog_json`，且不生成 `model-catalogs/` 目录。
6. profile 原有 `model_catalog_json`（用户自配）在映射为空时被原样保留。
7. 清除中转配置后 `config.toml` 不再包含 `model_catalog_json`（回归既有行为）。
8. Claude 目标 profile 不写 `model_catalog_json`，也不生成 `model-catalogs/` 目录。
9. 目录文件先于 `config.toml` 落盘：不存在“config.toml 引用了尚未写出的目录文件”的中间态。
10. 映射解析与去重规则与 `crate::model_catalog::relay_profile_catalog_models` 完全一致，未引入第二套解析实现。
11. 未新增或改动 `assets/inject/` 下任何文件，未新增任何 DOM / 状态 / `model/list` 注入路径。

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
