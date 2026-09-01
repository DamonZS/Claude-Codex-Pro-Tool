# Multica 独立活动资源验收

验证对象：`spec/multica-activities.md`

## 通过标准
1. workspace bootstrap/query 暴露独立 `activities` 集合。
2. 活动字段只读，renderer 不提供写入表单。
3. 活动详情不会作为 HTML/Markdown 执行。

## 验证方式
```powershell
cargo test -p claude-codex-pro-core --lib multica_workspace -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
cargo fmt --check
```
