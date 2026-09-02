# Codex 原生工作流状态投影验收

对应规格：`spec/codex-native-workflow-inventory.md`

## 通过标准

- Bootstrap 和 Query 暴露 `codex_native_inbox`、`codex_native_automations`、`codex_native_chat_sessions`。
- 数据库存在对应表时可读取摘要；缺表/缺列时返回空集合而不是失败。
- 自动化记录包含受限 `runs` 结果摘要，且按真实 `automation_id` 关联。
- 会话目录项的 `id == thread_id`，同一 thread 不因两个本机数据库来源重复出现。
- 前端“我的任务”显示三类原生状态及来源说明。
- Rust 单元测试与 `cdp_bridge` 测试通过，Release 构建成功。

## 验证方式

```powershell
cargo test -p claude-codex-pro-core --lib multica_workspace -- --nocapture
cargo test -p claude-codex-pro-core --test cdp_bridge -- --nocapture
node --check assets/inject/renderer-inject.js
cargo build --release
```

## 非目标检查

不验证云端 Multica API 的通知写入、跨工作区同步和计费统计；本次只验证 Codex 本机只读投影。
