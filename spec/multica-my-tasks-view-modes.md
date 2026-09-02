# 我的任务多视图模式

## 背景

上游 Multica 的 `MyIssuesPage` 通过 `IssueSurface` 提供 board、list、table、swimlane 四种任务视图。本地“我的任务”目前只有 board，无法按列表、字段表格或分泳道方式扫描任务。

## 目标

- 在“我的任务”中提供 board、list、table、swimlane 四种模式。
- 四种模式都使用同一份已查询、已筛选的 Issue 数据，不复制或伪造任务状态。
- 模式切换只影响当前界面呈现，不改变任务数据。

## 非目标

- 不新增服务端执行器、daemon 或 Codex runtime。
- 不把只读原生快照变成可编辑任务。

## 功能要求

- 工具栏显示四个互斥模式按钮，并标记当前模式。
- board 保留现有拖放状态列行为。
- list 按任务逐行展示标题、状态、优先级、负责人和项目。
- table 以稳定列展示上述字段及更新时间。
- swimlane 按状态分组，每组展示任务列表。
- 无任务时显示明确空态；模式切换不触发额外写入。

## 交付范围

- `assets/inject/renderer-inject.js` 模式状态、工具栏和三种渲染器。
- 对应验收文档和语法/前端构建验证。
