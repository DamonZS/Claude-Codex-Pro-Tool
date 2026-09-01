# Multica 独立标签资源验收

验证对象：`spec/multica-labels.md`

## 通过标准
1. bootstrap/query 暴露独立 `labels` 集合。
2. 标签名称非空且长度受限，资源类型仅允许 issue、agent、skill。
3. 颜色仅接受 `#RRGGBB`，非法颜色拒绝写入。
4. 通用 upsert/delete 遵守 revision CAS。

## 验证方式
```powershell
cargo test -p claude-codex-pro-core --lib multica_workspace -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
cargo fmt --check
```
