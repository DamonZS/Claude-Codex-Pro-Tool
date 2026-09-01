# Multica 独立活动资源

## 目标
按上游 Activity/Timeline DTO 提供独立 `activities` 集合，保存任务活动的操作者、动作、详情和时间，供本地控制面查看。

## 约束
活动是只读投影，不允许通用编辑器伪造写入；详情按 JSON/文本安全展示，不执行 HTML 或 Markdown。
