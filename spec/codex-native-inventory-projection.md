# Codex 原生会话与智能体只读投影

## 背景

“我的任务”当前只显示本地 Multica Issue，用户无法在同一工作区看到当前 Codex 页面已经展示的原生会话，也无法判断哪些本地智能体具备真实 Codex 执行映射。

## 目标

- 在“我的任务”看板之外增加一个紧凑的“Codex 原生会话”只读区域。
- 仅投影当前页面 DOM 中带有稳定 `data-app-action-sidebar-thread-id` 属性的真实会话行；不得扫描 React store、改写原生导航或伪造会话。
- 展示本地 Multica 智能体中具有有效 Codex 执行绑定的条目，并显示当前页面是否支持 subagent；没有证据时显示空态。
- 投影失败或页面导航变化时保留任务看板，不影响 Codex 原生页面。

## 非目标

- 不新增或猜测 `thread/list`、`agent/list` 等未被当前 Codex Host 暴露的协议。
- 不把原生会话伪装成 Multica Issue，不自动创建任务或执行对象。
- 不修改 Codex 原生项目、会话、模型和导航。

## 数据与交互

- 原生会话字段仅包括稳定 thread ID、可见标题、所属项目标签（若 DOM 提供）和当前激活状态。
- 点击会话使用其原生 DOM 行的现有激活行为；找不到行时显示不可用状态。
- 智能体投影仅使用已查询的本地 `agents` 集合和执行绑定中的稳定 agent ID；凭据、提示词和路径不得进入 DOM。
- 页面每次刷新看板时重新读取投影，最多读取 100 条会话。
- 原生项目与线程关联不得只依赖可选的 `threads.project_id`：当 SQLite 提供
  `project_roots.path` 和线程 `cwd` 时，按规范化后的绝对路径执行大小写不敏感的
  精确/子目录匹配，并将最长匹配项目写入线程的只读 `project_id`、`project_path`；
  无匹配时保留线程但不伪造项目归属。
- 当本机 Codex 状态库提供线程父子关系时，将子线程投影为只读原生子智能体；
  不创建 Multica Agent、不推断执行绑定。
- 从 `~/.codex/skills/<name>/SKILL.md` 读取只读名称、标题和描述元数据，作为
  原生 Skills 清单；不得把 Skill 正文、凭据或运行路径写入可编辑实体。
- 从 `thread_history_1.sqlite/thread_items` 投影 `codex_native_events`，字段限于线程、事件 ID、类型、序号、时间和最多 160 字符摘要，供任务审计与对账使用。
- 原生事件索引不得包含完整消息正文、工具参数、命令输出或凭据；读取始终使用 SQLite 只读连接并限制最多 1000 条。
- 本地实体编辑器必须以 Multica 上游 wire 字段为主（Issue、Project、Agent、Squad、Autopilot），并兼容既有本地 camelCase 别名；保存时不得丢弃上游已返回的安全业务字段。
