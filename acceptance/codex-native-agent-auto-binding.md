# Codex 原生智能体自动绑定验收

对应规格：`spec/codex-native-agent-auto-binding.md`

## 必须通过

- Rust 单元/集成测试覆盖 subagent fork、幂等重放、unsupported 降级和普通 thread 回归。
- 前端类型检查和 Vite 构建通过。
- 手动在 Codex 页面打开父会话，在任务卡选择已分配 Agent 执行，确认执行记录显示 Agent、父/子 thread 和 attempt，点击“打开对话”进入子会话。

## 验证命令

```powershell
cargo test -p claude-codex-pro-core --test bridge_routes multica_execution -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
```
