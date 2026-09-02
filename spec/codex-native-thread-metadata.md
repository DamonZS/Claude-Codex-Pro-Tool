# Codex 原生会话元数据投影

## 背景

Codex 本机 `state_5.sqlite` 的 `threads` 表保存了会话归档、置顶、模型、来源、Git 分支和项目关联。当前“我的任务”只投影标题、路径和更新时间，导致用户无法准确判断原生会话状态。

## 目标

在只读原生会话投影中保留数据库真实字段，并通过现有项目关联逻辑展示原生项目 ID。不得把原生会话转为可编辑 Multica Issue，也不得复制提示词或会话正文。

## 要求

- 投影稳定 `id`、`title`、`cwd`、`updated_at_ms`。
- 在字段存在时投影 `project_id`、`archived`、`is_pinned`、`model`、`model_provider`、`source`、`git_branch`、`git_origin_url`、`agent_nickname`。
- 不同 Codex schema 缺少可选列时使用空值，不得导致整个 bootstrap 失败。
- 原生投影继续标记 `source=codex_native` 并保持只读。

## 非目标

- 不写入 Codex SQLite。
- 不伪造项目或会话状态。
- 不将 `first_user_message`、rollout 路径或会话正文返回到前端。
