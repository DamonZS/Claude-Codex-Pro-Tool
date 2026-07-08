# 第三方供应商导入完整性修复

## 背景

当前从 cc-switch 等第三方工具导入供应商时，管理工具只识别 Codex 类型配置，并且导入后前端会重新生成默认 Codex 配置，导致原始路由、API 格式、Claude 配置、模型映射、插件配置与部分 API Key 来源丢失。用户在供应商配置页无法看到第三方配置的来源应用、路由类型和不同 API 格式。

## 目标

本次要完成：

- 从 cc-switch 导入 `codex`、`claude`、`claude-desktop` 三类供应商配置。
- Codex 导入时保留原始 `settings_config.config` 和 `auth` 内容，不被前端默认配置覆盖。
- Claude / Claude Desktop 导入时读取 `env` 中的 base URL、token、默认模型和 Claude 模型映射。
- 识别 Codex 配置中的 `http_headers.Authorization = "Bearer ..."` 作为 API Key 来源。
- 供应商页展示第三方导入来源、目标应用/路由、API 格式、Claude 模型映射等可审查信息。
- 增加 Anthropic / Claude 原生预设模板。
- 对齐 cc-switch 的 Claude Desktop 路由语义：Direct 直连、Proxy 本地路由、API 格式与 `claude-*` 安全路由 ID 到上游模型的映射。
- 不在日志、文档、测试输出中暴露真实密钥。

本次不包含：

- 不重构供应商整体架构。
- 不改变 Codex/Claude 启动与中文注入链路。
- 不删除或重置用户已有供应商数据。
- 不新增第三方依赖。

## 用户视角描述

用户点击“从 cc-switch 导入”后，应看到 Codex、Claude、Claude Desktop 相关供应商均可导入。导入后的供应商卡片和编辑页能说明配置来源、应用路由和 API 格式；对于 Claude 配置，能看到 Sonnet / Opus / Haiku / Fable 等模型路由；对于 Codex 配置，原始 config 中的插件、provider、路由等内容不会被丢弃。

用户新建供应商时，应能从预设模板选择 Anthropic / Claude，并能在 Claude / Claude Desktop 配置区域选择 Anthropic Messages、OpenAI Chat Completions、OpenAI Responses API、Gemini Native generateContent 等格式。

## 功能要求

- 导入命令扫描 cc-switch `providers` 表中 `app_type IN ('codex','claude','claude-desktop')` 的记录。
- Codex 配置：
  - 保留原始 TOML config 到 `configContents`。
  - 保留 auth JSON 到 `authContents`。
  - API Key 读取优先级至少覆盖 auth、`experimental_bearer_token`、provider `env_key`、`http_headers.Authorization`。
- Claude 配置：
  - 从 `env.ANTHROPIC_BASE_URL` 读取请求地址。
  - 从 `env.ANTHROPIC_AUTH_TOKEN` 读取 API Key。
  - 从 `ANTHROPIC_MODEL` 和 `ANTHROPIC_DEFAULT_*_MODEL` 读取模型与映射。
  - 在元信息中标记 API 格式，默认 `Anthropic Messages`。
- Claude Desktop 路由：
  - 支持 `claudeDesktopMode`：`direct` / `proxy`。
  - Direct 直连只表示 Anthropic Messages 原生协议，不需要模型路由。
  - Proxy 本地路由需要开启“是否开启路由”，并使用 `claude-sonnet-*` / `claude-opus-*` / `claude-haiku-*` / `claude-fable-*` 作为安全路由 ID。
  - 模型路由应显示：角色、路由 ID、菜单显示名、实际请求模型、是否支持 1M。
  - 从 cc-switch 导入时优先保留 `meta.apiFormat`、`meta.claudeDesktopMode`、`meta.claudeDesktopModelRoutes`；若没有显式路由，则按 `ANTHROPIC_DEFAULT_*_MODEL` 派生默认路由。
  - 支持 `[1M]` 后缀转换为 `supports1m` 标记，实际请求模型中不保留该后缀。
- API 格式选项：
  - `Anthropic Messages（原生）`：不需要路由。
  - `OpenAI Chat Completions（需开启路由）`：需要 Claude Desktop Proxy 路由。
  - `OpenAI Responses API（需开启路由）`：需要 Claude Desktop Proxy 路由。
  - `Gemini Native generateContent（需开启路由）`：需要 Claude Desktop Proxy 路由。
- 前端导入后不得再次调用默认配置生成覆盖后端导入内容。
- 供应商编辑页应展示：导入来源、目标应用、API 格式、是否开启路由、模型映射/模型列表。

## UI / 交互要求

- 供应商卡片左侧拖拽柄必须可以在当前目标应用过滤列表内完成拖拽排序；松手后必须保存到真实供应商顺序，且不使用整卡片 HTML5 draggable，避免与 Tauri WebView 原生 drag/drop 事件冲突。
- 供应商卡片左侧拖拽柄必须可以在当前过滤列表内完成拖拽排序；松手后必须保存到真实供应商顺序，且不能与 WebView 原生 drag/drop 事件冲突。
- 供应商卡片显示第三方来源标签，例如 `cc-switch · Claude · Anthropic Messages`。
- 编辑页新增或完善“第三方导入信息 / Claude Desktop 路由”区域，展示来源、目标应用、API 格式、路由说明和模型映射。
- API 格式下拉应包含 cc-switch 的四种 Claude API 格式，并在需要路由的格式上明确提示。
- 路由开关文案使用“是否开启路由”。
- 若配置为 Claude-only，应清楚说明其用于 Claude 路由/共享配置，避免误认为 Codex Responses 配置。
- API Key 字段继续按现有方式隐藏/编辑，只有用户点击眼睛时才显示。

## 数据与接口要求

- 继续复用现有 `import_ccswitch_codex_providers` Tauri 命令以保持前端兼容，但命令语义扩展为导入 cc-switch 支持的供应商。
- `RelayProfile` 可增加向后兼容的可选元字段：`claudeDesktopMode`、`routeEnabled`。
- 老配置缺少新字段时必须可正常反序列化。
- `modelMappingJson` 应兼容数组形式和 cc-switch route map 语义，至少能表达 `routeId`、`requestModel`、`displayName`、`supports1m`。

## 技术约束

- 保持最小改动。
- 不重置 `settings.json`、不删除已有 profile。
- 不输出真实密钥。
- 不影响盘古记忆、Claude 中文注入和现有供应商切换逻辑。

## 交付范围

- 规格文档与验收文档。
- 后端 cc-switch 导入解析增强。
- 前端供应商页导入保真、Anthropic 预设、API 格式与路由展示。
- 定向测试与前端构建验证。

