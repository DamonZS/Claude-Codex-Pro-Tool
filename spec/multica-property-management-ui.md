# 原始属性管理与视图偏好 UI 闭环

## 背景与目标

补齐完整移植中的属性目录 UI。固定上游 `9fce92f427694d7d303258aa281b05c902a95ba9`
已有 PropertiesTab；本地 Core 已提供属性定义、任务值与用户视图偏好的持久化接口。
本任务把原始控件接入我的任务二级页面，保留三个顶级模块、NativeSubtasks 和时间线。

## 用户与交互

- 我的任务提供“管理属性”二级入口，路径 `/{workspace}/my-issues/properties`。
- 二级页有明确“返回我的任务”按钮；直接深链和 bootstrap 加载/错误时也能返回集合。
- 使用固定版本 PropertiesTab 的创建、编辑、选项、归档/恢复、搜索与显示归档控件。
- 属性读取失败显示错误及重试，不显示成空目录。保留原始加载态、空态和错误 toast。
- 原始 Issue 属性控件可新增值、修改与清空；归档定义保留已有值，仍可清空。
- 原始管理视图的隐藏与排序使用真实 `builtin:*` 和 `view:<id>`，重载保留。

## 数据与能力

- Core bootstrap 增加 `permissions.managePropertyCatalog`，来源为现有本地工作区
  启用策略；与 workspace/upsert 的启用检查一致，不授予上游 owner/admin 身份。
- `/api/config.feature_flags.local_property_catalog_management` 仅接受该明确布尔能力。
  缺失/false 时管理页只读；member 身份不变。后端继续独立校验启用、类型、CAS。
- 原始组件通过可选本地管理能力和说明文案插槽适配；未传入时保留上游权限规则。
- 使用现有 properties、issue properties、issue_view_preferences API；归档不物理删除定义。
- 原始选项编辑器的新选项使用空 ID；adapter 为每个新选项产生稳定的非空 ID，
  保留既有选项 ID，并使相同命令在 adapter 重建后恢复同一结果。

## 技术与交付

修改独立规格/验收、main 路由、adapter 能力映射、固定源码生成器及必要闭包/哈希，
新增原始 UI 点击 fixture 测试与能力映射测试。保留现有修改和用户数据，不修改 renderer。
不运行真实 UI；由父线程在最终 Release 后验证。构建顺序由父线程控制，本轮先完成
vendor、tsc、Vitest；Vite → Rust → Release 由父线程统一执行以避免嵌入资源竞争。
