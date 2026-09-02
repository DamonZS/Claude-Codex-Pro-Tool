# Codex 默认中转供应商删除

## 背景

供应商管理原先只从 CCP 设置列表删除 Profile。若被删 Profile 正是活动 Codex 供应商，Codex 的 `config.toml` 和 `auth.json` 仍然保留旧中转配置，独立启动时会继续调用已删除的供应商。

## 目标

- 删除活动 Codex Profile 前清除 live API 模式，恢复官方 Codex 配置。
- 删除完成后从 CCP 设置中移除该 Profile，活动 ID 按现有逻辑重选。
- 删除非活动 Profile 或 Claude/Claude Desktop Profile 时保持原有行为。

## 非目标

- 不删除用户的 Codex 会话、项目、日志、hook 或其他状态文件。
- 不删除 `instruction-inject.sh` 或修改内部消息隔离逻辑。
- 不改变“清除 API 模式”命令本身的备份和原子写入实现。

## 交付范围

- 供应商管理删除流程的活动 Codex 分支。
- 对应源码契约测试和本地验证。
