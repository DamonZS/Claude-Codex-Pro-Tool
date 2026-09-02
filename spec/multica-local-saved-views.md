# Multica 我的任务本机保存视图

## 背景

上游 Multica 的保存视图属于服务端 `IssueView` 资源；当前本地控制面尚未实现该资源。任务页面已有筛选、显示模式和紧凑布局，但刷新或重新打开后会丢失选择。

## 目标

- 将当前任务 scope、显示模式和紧凑布局保存到本地控制面 `issue_views` 资源，并以 localStorage 作为兼容缓存。
- 支持选择、覆盖同名、删除本机保存视图。
- 明确标注为“本机视图”，不声称已同步上游服务端。

## 功能要求

- 允许保存 `all`、`assigned`、`created`、`agents`、`working` 五种 scope；`working` 仅包含本地执行绑定中非终态任务。
- 允许保存 `board`、`list`、`table`、`swimlane` 四种显示模式及 `boardCompact`。
- 读取 localStorage 时校验结构；损坏、非数组或缺少 id/name 的记录忽略。
- localStorage 不可写时显示明确错误，不阻断任务查看和编辑。
- 删除只删除本机控制面记录，并同步兼容缓存，不调用上游 Multica 服务端。

## 非目标

- 不伪造上游服务端 `IssueView` 同步；本地 `issue_views` 明确属于 CCP 控制面。
- 不启动额外 Codex runtime、daemon 或执行器。
