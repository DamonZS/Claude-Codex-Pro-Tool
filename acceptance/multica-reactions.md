# Multica 独立评论反应验收

验证对象：`spec/multica-reactions.md`

## 通过标准
1. bootstrap/query 暴露独立 `reactions` 集合。
2. `comment_id`、`actor_id`、非空 emoji 均受校验。
3. 通用 upsert/delete 继续执行 revision CAS。

## 验证方式
```powershell
cargo test -p claude-codex-pro-core --lib multica_workspace -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
cargo fmt --check
```
