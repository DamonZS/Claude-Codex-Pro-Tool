# 设置页精简与供应商 cc-switch 风格改造

## 背景

管理工具设置页曾展示部分高级配置卡片，供应商页也存在卡片拖拽排序体验不稳定、供应商显示与按钮样式不够贴近 cc-switch 的问题。用户要求删除设置页指定卡片，并让供应商列表、拖拽体验、Claude 供应商编辑页结构对齐 `H:\xunlei\cc-switch-main`，但必须保持管理工具现有深色主题与整体色调。

## 目标

本次要完成：

- 从设置页删除 `Codex 启动参数`、`图片覆盖`、`盘古记忆`、`安全边界` 四个独立卡片。
- 保留设置页其它卡片和已有保存逻辑。
- 修复供应商页供应商卡片拖拽排序，使拖拽源在 Tauri WebView 中稳定可识别。
- 拖拽过程显示跟随鼠标移动的卡片镜像，而不是只在松手后瞬间换序。
- 保持拖拽排序保存到现有 `relayProfiles` 顺序的逻辑不变。
- 供应商卡片显示与按钮样式对齐 cc-switch：轻量圆角卡片、左侧拖拽柄、供应商图标、名称与官网/URL 分层展示、右侧轻量图标按钮和高亮“使用”按钮。
- Claude / Claude Desktop 供应商编辑页按 cc-switch Claude 编辑页结构展示：返回标题、基础信息、API Key 眼睛按钮、请求地址、完整 URL 标识、高级选项、API 格式、认证字段、模型映射、默认兜底模型、User-Agent、Header/Body 覆盖、配置 JSON、连通检测配置、计费配置、底部保存条。
- Claude 编辑页只对齐结构和交互，不改变管理工具现有深色主题。

本次不包含：

- 删除盘古记忆其它页面能力。
- 删除后端设置字段、Tauri command 或配置文件字段。
- 重写供应商管理架构。
- 修改 Claude 中文注入脚本。
- 新增 UI 依赖。

## 用户视角描述

用户进入设置页后，不再看到 `Codex 启动参数`、`图片覆盖`、`盘古记忆`、`安全边界` 这几个卡片。用户进入供应商页后，可以按 `Codex / Claude / Claude Desktop` 过滤供应商；卡片显示接近 cc-switch，名称下方展示官网或接口地址，右侧按钮清晰；按住左侧拖拽柄时，卡片镜像跟随鼠标移动，松手后顺序保存。用户编辑 Claude 供应商时，看到与 cc-switch Claude 供应商编辑页一致的信息结构，并可保存 API Key、路由、模型映射等配置。

## 功能要求

- 设置页仍显示设置文件位置、Codex 增强矩阵、Claude 一键汉化、CLI Wrapper 和运行日志等保留内容。
- 被删除的四个卡片不得再由 `SettingsScreen` 渲染。
- 供应商拖拽开始时使用 pointer 事件和 pointer capture，避免 WebView 丢失拖拽源。
- 拖拽移动时实时预览排序，并更新跟随鼠标的镜像卡片位置。
- 释放拖拽后调用现有 `saveSupplierOrder`，并通过 `actions.saveSettings` 保存 `relayProfiles` 顺序。
- 如果拖拽保存失败，仍使用现有逻辑回滚到原顺序。
- 从第三方导入的供应商配置必须保留 API Key、目标应用、API 格式、路由、模型映射和原始配置内容。
- Claude 供应商编辑页必须保存新增的可编辑字段，避免导入配置在编辑保存后丢失。

## UI / 交互要求

- 供应商卡片使用 cc-switch 式层级：圆角边框卡片、左侧 `GripVertical` 拖拽柄、供应商头像、名称、状态徽标、URL、右侧操作按钮。
- 普通供应商卡片默认不展示冗余协议摘要，只保留名称、徽标与 URL；聚合供应商可展示成员/策略摘要。
- 不显示“不写 API 文件”等误导文案；缺少地址时显示“未配置接口地址”。
- 右侧动作图标顺序对齐 cc-switch：使用、编辑、复制、连通检测、用量、删除。
- 图标按钮应是轻量透明按钮，hover 时出现浅色底，不使用厚重边框按钮。
- “使用”按钮使用蓝色高亮按钮，大小和间距接近 cc-switch。
- 拖拽柄必须有明确抓取光标与标题提示。
- 拖拽镜像必须固定定位并跟随鼠标移动，原卡片作为占位显示。
- Claude 编辑页顶部为返回按钮 + `编辑供应商` 标题。
- Claude 编辑页基础区包含：供应商名称、备注、官网链接、API Key、请求地址、完整 URL 标识和提示文案。
- Claude 编辑页高级区包含：API 格式、认证字段、模型映射、默认兜底模型、自定义 User-Agent、本地代理请求覆盖、配置 JSON、连通检测配置、计费配置。
- Claude 编辑页底部必须有 sticky 保存条。
- Claude 编辑页色调沿用管理工具深色主题，不套用 cc-switch 浅色背景。

## 数据与接口要求

- 继续使用现有 `actions.saveSettings` 保存 `relayProfiles` 顺序与字段。
- 不新增 Tauri command。
- 保持 settings schema 向后兼容；新增前端可编辑字段必须有 serde 默认值。
- 导入或编辑 Claude 配置时不得丢失 API Key。

## 技术约束

- 优先修改 `apps/claude-codex-pro-manager/src/screens.tsx`、`apps/claude-codex-pro-manager/src/styles.css`、`apps/claude-codex-pro-manager/src/lib/supplier.ts` 和必要类型/设置结构。
- 回归测试放在 `apps/claude-codex-pro-manager/src-tauri/tests/windows_subsystem.rs`。
- 不修改 `assets/inject/claude-chinese-inject.js`。
- 不终止 Codex 进程；如果 debug manager 构建被占用，只允许终止 `claude-codex-pro-manager.exe`。

## 交付范围

- 设置页 UI 渲染调整。
- 供应商卡片显示、按钮样式与拖拽体验调整。
- Claude 供应商编辑页 cc-switch 结构对齐。
- 必要类型、设置字段和导入字段保留逻辑。
- 回归测试更新。
- 本规格文档与对应验收标准。
