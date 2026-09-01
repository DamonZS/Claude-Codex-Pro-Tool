# Multica 本地任务租约与转录索引验收

对应规格：[spec/multica-task-lease-transcript.md](../spec/multica-task-lease-transcript.md)。

## 通过条件

- 同一有效租约允许持有者续约和释放；不同令牌在到期前不能接管，到期后可接管。
- 终态 binding 不可领取或续约，revision 冲突和令牌冲突返回稳定错误。
- 转录消息按 `(binding_id, seq)` 幂等，冲突消息被拒绝，列表升序且分页受限。
- 持久化 JSON 不含受限制正文，状态校验拒绝重复序号、未知 binding、超长字段及非法租约。

## 必需证据

```powershell
cargo fmt --check
cargo test -p claude-codex-pro-core multica_execution_store -- --nocapture
git diff --check
```

本切片不要求启动 daemon 或 `codex.exe app-server`；运行时仅验证本地状态不变量。
