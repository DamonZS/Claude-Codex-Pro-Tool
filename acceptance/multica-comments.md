# Multica 独立评论资源验收

验证对象：`spec/multica-comments.md`

## 通过标准
1. workspace bootstrap/query 暴露独立 `comments` 集合。
2. 评论可通过通用本地 upsert/delete 写入，revision 冲突被拒绝。
3. 不存在任务、空正文、NUL、超长正文和 `system` 类型写入失败。
4. renderer 评论编辑器使用文本节点展示正文，不执行 Markdown/HTML。

## 验证方式
```powershell
cargo test -p claude-codex-pro-core --lib multica_workspace -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
cargo fmt --check
git diff --check
```
