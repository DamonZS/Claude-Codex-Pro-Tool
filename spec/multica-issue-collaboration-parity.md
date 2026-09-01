# Multica Issue 协作字段对接

## 背景

上游 Multica 的 Issue DTO 不止标题、状态和分配信息，还包含流程元数据、自定义属性、标签、反应和活动时间。嵌入式工作区必须保留这些上游字段，并在没有对应本地协作 API 时明确区分可编辑字段与只读投影。

## 目标

- Issue 编辑器读写上游 `metadata` 与 `properties` 对象，不因旧 camelCase 别名归一化而丢失字段。
- Issue 卡片/通用详情显示 `status_category`、`status_name`、标签、反应和最近活动时间的安全摘要。
- JSON 输入在保存前必须解析为对象或数组；无效 JSON 阻止写入。
- 评论正文、source context、凭据和其他潜在指令内容不进入通用摘要渲染。

## 上游依据

- `packages/core/types/issue.ts`：`metadata`、`properties`、`labels`、`reactions`、`last_activity_at`、`source_context`。
- `packages/core/types/comment.ts` 与 `server/internal/handler/comment.go`：评论是独立资源，包含正文、线程、解析状态与反应，不能伪装成 Issue 字段。

## 非目标

- 本阶段不伪造评论/标签/订阅 HTTP API。
- 不渲染不可信 Markdown/HTML，不把 source context 的历史正文当作执行提示词。

## 交付

- renderer Issue 编辑器 JSON 字段和校验。
- 安全只读摘要字段。
- CDP 契约测试与对应验收文档。

## 本地关联投影补充

查询 `issues` 或 `my_tasks` 时，必须把本地独立集合中的真实 `labels`、`reactions`、
`comments` 和 `activities` 按 `issue_id`/`comment_id` 关联到 Issue 只读响应，提供
上游 Issue 类型中的 `labels`、`reactions`、`last_activity_at` 以及稳定计数字段。
这些字段只能派生于已保存实体，不得写回存储或从不存在的远端数据推断。
