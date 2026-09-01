# Multica Project Resource 独立资源对接

## 背景

上游 Multica 将项目资源建模为独立的 `ProjectResource`，通过项目资源 API 进行分页查询、创建、更新和删除；它不是 Project DTO 中可随意覆盖的嵌套字段。

## 目标

- 在嵌入式 workspace 中提供独立 `project_resources` 集合。
- 保留上游 `project_id`、`resource_type`、`resource_ref`、`label`、`position` 字段及 revision CAS。
- 只接受上游已确认的 `github_repo` 与 `local_directory` 类型，并校验其引用结构。
- 旧 workspace JSON 没有该字段时自动按空集合兼容读取。

## 非目标

- 不启动 Multica daemon，不复制上游 HTTP 服务端或数据库。
- 不把本地目录当作可执行入口；资源只保存指针和执行模式。

## 验收

- 查询返回独立 `project_resources` 资源。
- 合法 GitHub/local 资源可保存，未知类型和非法执行模式被拒绝。
- 缺失父 Project 的资源被拒绝，旧状态仍可读取。
- renderer 提供独立资源模块和编辑字段。
