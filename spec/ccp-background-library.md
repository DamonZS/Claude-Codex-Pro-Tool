# CCP 外观与背景图库

## 背景

主题中心同时展示 Codex 注入主题、DIY、在线下载、导入以及 CCP 管理工具单张背景，用户容易误认为这些操作作用于同一个客户端。现有 CCP 背景每次选择都会覆盖上一张，只保留一个不可直接切换的上一版本，不符合主题中心的收藏和复用场景。

## 目标

- 将页面分为“CCP 外观”和“Codex 主题”两个明确视图。
- CCP 外观提供本地高清背景图库，支持保存多张、预览、应用、恢复默认和删除。
- Codex 主题视图只展示 Codex 的 DIY、在线下载、导入和主题卡片。
- 两个视图具有独立标题、说明、工具栏和状态统计，任何 CCP 背景操作都不出现在 Codex 主题视图中。
- 自动迁移现有单张 CCP 自定义背景，不丢失用户数据。

## 非目标

- 不把 CCP 背景注入 Codex，也不把 Codex 主题 CSS 应用到 Manager 控件。
- 不上传背景、不提供在线背景市场、不保存来源绝对路径。
- 不开放任意 CSS 或 Manager 布局编辑器。

## 用户流程

1. 用户进入主题中心，使用清晰的分段控件切换“CCP 外观”和“Codex 主题”。
2. CCP 外观首张固定为默认外观，后续以三列卡片展示已保存背景。
3. 点击“添加背景”选择本地高清图片；图片通过校验后存入图库并立即应用。
4. 点击其他卡片立即切换 Manager 背景；当前卡片显示“正在使用”。
5. 点击默认外观只取消当前自定义背景，不删除图库。
6. 非当前背景可删除；当前背景必须先切换或恢复默认。
7. Codex 主题视图提供“DIY 主题”“下载主题”“导入主题”和制作指南，不出现 CCP 背景操作。

## 功能要求

- 支持 PNG、JPEG、WebP；至少 1920×1080，最大 16 MiB、100,000,000 像素。
- 使用内容哈希生成稳定本地 ID；重复选择同一图片不重复保存，只切换到已有项。
- 图库元数据包含 ID、原文件名、MIME、宽高、哈希和更新时间，不包含来源绝对路径。
- 列表只返回压缩预览，当前 Manager 背景仍按需返回可用于运行时的完整数据。
- 旧 `current.bin` 与状态元数据在首次访问时迁移到图库；迁移成功前不得删除旧文件。
- 新增、切换、取消启用和删除均在主题仓库锁下执行；失败不改变当前背景选择。
- 恢复默认保留所有背景；删除当前背景必须被拒绝。

## UI 与交互

- 顶部使用“CCP 外观 / Codex 主题”分段控件，不把 Agent 过滤器当作该分区控件。
- CCP 外观使用一行三张背景卡片；默认卡固定第一张，显示清晰预览、名称、分辨率和状态。
- CCP 默认卡明确表示未启用图库背景；恢复默认不得清空或覆盖已经保存的背景。
- “添加背景”是 CCP 外观工具栏主操作；删除使用垃圾桶图标和 Tooltip。
- Codex 视图把“新建主题”更名为“导入主题”，避免与 DIY 创建混淆。
- 两个视图分别提供加载、空态、错误、忙碌、当前和禁用状态。

## 数据与接口

新增 Core 结构和 API：

```rust
CodexManagerBackgroundItem
CodexManagerBackgroundLibrary

CodexThemeStore::manager_background_library()
CodexThemeStore::apply_manager_background(background_id)
CodexThemeStore::delete_manager_background(background_id)
```

`set_manager_background` 改为去重保存并应用；`clear_manager_background` 只取消启用。新增 Tauri 命令：

```text
list_codex_manager_backgrounds
apply_codex_manager_background
delete_codex_manager_background
```

## 技术约束

- 沿用现有 Core 图片校验、仓库锁、原子写入和 Tauri `CommandResult`。
- 不修改供应商、Codex 注入、Claude、记忆、更新或其他导航行为。
- 不回滚在线主题下载和 DIY 工作台未提交改动。

## 交付范围

- Core 图库状态、旧数据迁移、去重、切换、删除和测试。
- Tauri 命令、前端类型与动作。
- 主题中心双视图、CCP 背景卡片和对应样式。
- 默认 Release 构建与原生窗口截图验证。
