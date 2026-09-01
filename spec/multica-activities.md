# Multica 独立活动资源

## 目标
按上游 Activity/Timeline DTO 提供独立 `activities` 集合，保存任务活动的操作者、动作、详情和时间，供本地控制面查看；Issue 查询同时投影只读 `timeline`，将匹配任务的评论与活动按 `(created_at, id)` 升序合并。

## 约束
活动是只读投影，不允许通用编辑器伪造写入；详情按 JSON/文本安全展示，不执行 HTML 或 Markdown。`timeline` 为派生字段，不改变各集合的 revision/CAS。
