# 供应商模型映射投影到 Codex 原生模型目录

## 背景

管理工具的「供应商与路由 → 编辑供应商 → 模型映射」在 Codex 目标下允许用户逐行填写「菜单显示名 / 实际请求模型 / 上下文窗口」，界面提示明确写着“菜单显示名用于 Codex 模型选择”。

当前实现只把映射写入 CCP 自己的设置（`RelayProfile.codexCatalogJson`）供管理工具内部展示，从不写进 Codex 的 `config.toml`。因此：

- 用户在 Codex 供应商上配置了 5 个映射模型（`deepseek-v4.1-flash`、`deepseek-v4-flash-vision-exp`、`deepseek-v4-pro`、`glm-5.3`、`glm-5.3-flash`）。
- `C:\Users\Damon\.claude-codex-pro\settings.json` 中该 profile 的 `codexCatalogJson` 确实是这 5 行。
- 但 `C:\Users\Damon\.codex\config.toml` 中没有 `model_catalog_json`，也没有 `model-catalogs/` 目录。
- Codex 原生「选择模型」下拉只显示内置模型（`gpt-5.6-sol` / `gpt-5.6-terra` / `gpt-5.6-luna` / `gpt-daybreak-*` / `gpt-5.5` / `gpt-5.2` / `codex-auto-review`），看不到这 5 个映射模型。

即：用户数据没有丢，只是从未投影到 Codex 原生模型目录机制上。

## 目标

- Codex 目标的 relay profile 在应用/切换时，若其模型映射非空，则由后端生成 Codex 原生模型目录文件 `<codex-home>/model-catalogs/relay-<profile-id>.json`，并把 `config.toml` 根键 `model_catalog_json` 指向该文件。
- 目录文件内容来自映射表：`slug` 用「实际请求模型」，`display_name` 用「菜单显示名」。
- 映射为空时不写 `model_catalog_json`，Codex 继续使用内置目录。
- 切换到无映射的供应商或退出中转模式后，不残留指向本供应商生成文件的 `model_catalog_json`。
- 目录文件必须先落盘再写 `config.toml`：Codex 在 `model_catalog_json` 指向的文件缺失时会直接启动失败。

## 非目标

- 不恢复任何形式的 Codex 原生模型选择注入（DOM 扫描、状态补丁、`model/list` 补丁、请求模型覆盖）。`assets/inject/renderer-inject.js` 不在本次交付范围。
- 不修改 Codex 官方应用安装目录下的任何文件。
- 不合并 Codex 内置目录（本次采用“映射即目录”的替换语义，见技术约束中的取舍说明）。
- 不改变「应用预设」清空既有映射的现状，也不改变 Codex 目标下「获取模型」不自动写入映射表的现状。
- 不改动供应商 Base URL、API Key、协议与路由逻辑。
- 不删除用户自有的 `model_catalog_json` 取值或用户自有的目录文件。

## 用户视角描述

1. 打开「供应商与路由 → 编辑供应商」，目标应用为 Codex，在「模型映射」中填写 5 行，例如菜单显示名 `DeepSeek V4.1 Flash`、实际请求模型 `deepseek-v4.1-flash`。
2. 保存并让该供应商成为 Codex 当前供应商。
3. CCP 生成 `<codex-home>/model-catalogs/relay-<profile-id>.json`，并在 `config.toml` 写入 `model_catalog_json = "<该文件绝对路径>"`。
4. 重启 Codex 后，原生「选择模型」下拉出现这 5 个模型，显示名为「菜单显示名」，选中后实际发送给上游的是「实际请求模型」。
5. 切回官方模式或切到一个没有模型映射的供应商后，`model_catalog_json` 被移除，Codex 恢复内置模型列表。

## 功能要求

- 生成条件：profile 的目标应用不是 Claude 系列，且 `codex_catalog_json` 能解析出至少一个非空 `model`。
- 解析规则：接受 `model` / `displayName` 或 `display_name` / `contextWindow` 或 `context_window`；`model` 为空的行跳过；重复 `slug` 只保留第一条。
- 显示名回退：`displayName` 为空时用 `model` 自身。
- 上下文窗口：解析出大于 0 的整数时，条目写入 `context_window` 与 `max_context_window`；否则不写这两个字段。
- 目录条目字段固定为：`slug`、`display_name`、`description`（取显示名）、`supported_reasoning_levels`（low/medium/high/xhigh 四档）、`shell_type` = `unified_exec`、`visibility` = `list`、`supported_in_api` = `true`、`priority`（从 1 递增）、`support_verbosity` = `true`、`truncation_policy` = `{"mode":"bytes","limit":10000}`、`experimental_supported_tools` = `[]`、`base_instructions` = `""`。
- 目录文件顶层结构必须是对象 `{"models":[...]}`，不能是裸数组。
- 目录文件编码为 UTF-8，不带 BOM。
- 根键值为绝对路径；若 `<codex-home>` 不是绝对路径则跳过注入，不写目录文件。
- 本次有映射时，`config.toml` 中既有的 `model_catalog_json` 被覆盖为本 CCP 生成路径。
- 本次无映射时不改动 `config.toml` 中既有的 `model_catalog_json`（用户自配目录继续生效）。
- 清除中转配置时继续移除 `model_catalog_json` 根键（沿用既有行为，本次不改）。

## UI / 交互要求

- 不新增、不删除、不移动任何界面元素。
- 「模型映射」区块的提示文案、列头、placeholder、空态保持原样，本次只让后端行为与文案一致。
- 保存与切换后的成功/失败通知语义不变。

## 数据与接口要求

- 输入：`RelayProfile.codex_catalog_json`（管理工具已保存的映射数组）。
- 输出一：`<codex-home>/model-catalogs/relay-<sanitized-profile-id>.json`。
- 输出二：`config.toml` 根键 `model_catalog_json`。
- 文件名净化：profile id 只保留 `[A-Za-z0-9._-]`，其余字符替换为 `-`；结果为空时使用 `profile` 兜底。
- 错误处理：目录文件写入失败时整个应用流程失败，不得写出引用了不存在文件的 `config.toml`。
- 幂等：重复应用同一 profile 产生同样的目录文件与同样的根键值。

## 技术约束

- 复用 `crate::model_catalog::relay_profile_catalog_models` 作为唯一的映射解析来源，避免第二套解析规则。
- 注入点在 `crates/claude-codex-pro-core/src/relay_config.rs` 中三个 profile → config 的应用函数，位于写盘之前、合并公共配置与上下文限制之后。
- 取舍说明（替换 vs 合并）：本次选择“替换”语义，即当前供应商的映射列表就是 Codex 可见的模型集合。理由是它与「模型映射」的产品语义一致（供应商能提供什么就显示什么），并且不依赖在应用时定位并执行 `codex.exe`。Codex 内置模型在退出中转模式后自动恢复。合并内置目录列为后续可选增强，不在本次交付范围。
- 不得把 API key、Bearer token 写入目录文件、日志或测试夹具。
- `assets/inject/` 下资源不得改动。

## 交付范围

- `crates/claude-codex-pro-core/src/model_catalog.rs`：向 crate 内暴露映射解析结果，并提供目录 JSON 构造。
- `crates/claude-codex-pro-core/src/relay_config.rs`：目录文件写入、根键注入、三个应用函数接线。
- `crates/claude-codex-pro-core/tests/relay_config.rs`：新增覆盖映射投影、空映射、路径净化、清除回滚的测试。
- `spec/codex-native-model-catalog-from-supplier-mapping.md` 与 `acceptance/codex-native-model-catalog-from-supplier-mapping.md`。
- 同步修订 `spec/remove-codex-model-selection-injection.md` 与 `acceptance/remove-codex-model-selection-injection.md` 的非目标表述，避免新旧规格冲突。
