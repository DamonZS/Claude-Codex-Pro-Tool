# 验收标准：Codex 默认中转供应商删除

验证对象：`spec/codex-default-relay-profile-delete.md`

## 验收项

1. 删除活动 Codex Profile 时先调用 `clearRelayMode()`，再保存剔除 Profile 后的设置。
2. 删除完成后活动 Codex 配置不再指向被删中转 Profile，独立启动 Codex 回到官方配置。
3. 删除非活动 Profile、Claude Profile 或 Claude Desktop Profile 不调用清除 API 模式。
4. 不删除用户会话、项目、日志、hook 或其他 Codex 状态数据。

## 必需验证

- `cargo fmt --check`
- `git diff --check`
- `npm --prefix apps/claude-codex-pro-manager run check`
- Manager 源码契约测试覆盖活动 Codex 删除顺序。
