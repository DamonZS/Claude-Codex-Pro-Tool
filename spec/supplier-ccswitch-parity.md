# 供应商配置与路由系统 cc-switch 对齐

## 背景

当前管理工具供应商页已经具备 Codex / Claude / Claude Desktop 三类供应商过滤、第三方导入、路由开关、模型映射和基础配置写入能力，但用户要求继续以 `H:\xunlei\cc-switch-main` 为基准，对照提取 cc-switch 中供应商前端、后端和路由语义，并应用到本项目供应商页。

目标是让管理工具在保留本项目整体色调与既有管理工具外壳的前提下，使供应商卡片、编辑页、模型映射、路由开关、Codex/Claude/Claude Desktop 配置写入与 cc-switch 行为尽量 1:1 对齐。

## 目标

本次工作包含：

- 对照 cc-switch 供应商页，补齐管理工具中 Codex、Claude、Claude Desktop 三类配置页的字段与交互语义。
- 供应商列表卡片结构对齐 cc-switch：拖拽柄、图标、名称、官网/Base URL、状态标签、使用/编辑/复制/删除等操作。
- 路由系统按目标应用分组：Codex 单独一套路由；Claude 与 Claude Desktop 共用 Claude 路由组。
- Claude / Claude Desktop 模型映射对齐 cc-switch：Sonnet、Opus、Fable、Haiku、Subagent 角色；显示名称、实际请求模型、声明支持 1M；实际请求模型支持获取模型后下拉填入。
- Codex 配置页支持 cc-switch 语义字段：API 格式、协议路由、模型列表、User-Agent、Header/Body 覆盖、配置预览。
- 后端读取和转换 cc-switch 导入配置时，保留 key、目标应用、API 格式、路由模式、模型映射、官网链接和原始配置内容。
- 后端协议代理保持 cc-switch 路由语义：Claude 模型安全 ID 映射到上游模型；1M 标记只作为本地能力声明，不直接发给上游；Fable 未配置时优先回落 Opus。

## 非目标

- 不改变管理工具整体色调、导航结构和盘古记忆功能。
- 不删除用户已有供应商数据。
- 不重置 Codex / Claude / Claude Desktop 配置。
- 不直接复制 cc-switch 全项目依赖栈，不引入大规模 UI 框架迁移。
- 不修改 Claude 中文注入逻辑。

## UI / 交互要求

### 供应商列表

- 顶部提供目标应用切换：Codex、Claude、Claude Desktop。
- 路由开关放在左侧，且状态按路由组独立：Codex 单独；Claude 与 Claude Desktop 共用。
- 供应商名称下方展示官网链接或 Base URL，类似 cc-switch。
- 卡片包含拖拽柄，拖拽时卡片应跟随鼠标移动，松开后保存排序。
- 卡片 hover 时展示操作按钮：使用、编辑、复制、删除。
- 使用中的供应商有明显状态标签，不显示乱码。

### 编辑页

- Claude / Claude Desktop 编辑页使用 cc-switch 类似结构：API Key、Base URL、API 格式、高级选项、模型映射、默认兜底模型、User-Agent、请求覆盖、配置 JSON、模型测试配置、计费配置。
- 模型测试配置和计费配置默认折叠，可点击尖括号展开/收起。
- 保存按钮常驻底部，不被长配置 JSON 挤出视野。
- API Key 输入框带眼睛按钮，可查看/隐藏。

### 模型映射

- 列头明确对应：模型角色、显示名称、实际请求模型、声明支持 1M。
- `实际请求模型` 支持从已获取模型列表下拉选择。
- 选择后写入对应行 requestModel，并同步模型映射 JSON / 文本。
- 支持角色：Sonnet、Opus、Fable、Haiku、Subagent。

## 数据与接口要求

- cc-switch 导入必须保留 API Key，不得只导入 Codex 配置。
- 导入配置保留 `targetApp`、`apiFormat`、`claudeDesktopMode`、`routeEnabled`、`modelMappingJson`、`websiteUrl`、`configContents`、`authContents`。
- Claude / Claude Desktop 写入配置时应保留模型映射和路由模式。
- Codex 写入配置时应保留 Codex route / wire api 语义。

## 技术约束

- 优先复用当前 `RelayProfile`、`SupplierScreen`、`supplier.ts`、`commands.rs`、`protocol_proxy.rs`。
- 对照 cc-switch 源码，但不得引入不可控大依赖迁移。
- 保持现有前端 `npm check`、Vite 构建和 Rust 定向测试通过。

## 交付范围

- `apps/claude-codex-pro-manager/src/screens.tsx`
- `apps/claude-codex-pro-manager/src/styles.css`
- `apps/claude-codex-pro-manager/src/lib/supplier.ts`
- `apps/claude-codex-pro-manager/src-tauri/src/commands.rs`
- `crates/claude-codex-pro-core/src/protocol_proxy.rs`
- 相关测试与验收文档
