# Multica Issue 协作字段对接验收

验证对象：`spec/multica-issue-collaboration-parity.md`

## 通过标准

1. Issue 编辑器包含流程元数据和自定义属性 JSON 输入，已有对象以格式化 JSON 显示。
2. 输入非法 JSON 时保存被阻止，并显示明确错误；不会向 bridge 发起 upsert。
3. 上游 Issue 的状态分类、标签、反应和最近活动时间可作为文本摘要显示；正文/source context 不被当作 HTML 执行。
4. 现有 snake_case 与 camelCase 分配字段兼容性回归不退化。

## 验证方式

```powershell
cargo test -p claude-codex-pro-core --test cdp_bridge codex_multica_workspace -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
cargo fmt --check
git diff --check
```

## 非范围

评论线程、标签 CRUD、反应写入、订阅管理和活动分页仍需后续独立 API 对接；本阶段不得以本地字段模拟这些服务端资源。
