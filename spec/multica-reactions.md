# Multica 独立评论反应资源

## 目标
按上游 `CommentReaction` 结构提供独立 `reactions` 集合，保存评论、操作者和 emoji，并遵守本地工作区 CAS。

## 约束
emoji 非空且有长度上限；不伪造远端事件、通知或 WebSocket。
