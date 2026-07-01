# 验收标准：ccswitch 重复导入修复旧供应商记录

验证对象：`spec/supplier-ccswitch-reimport-repair.md`

## 验收项

1. 文档存在
   - 通过标准：本 spec 与验收文档均存在。
   - 证据：git diff 或文件存在检查。

2. 同 ID ccswitch 记录会被替换
   - 通过标准：前端导入逻辑对同 ID 且 ccswitch 来源的已有供应商执行替换，不再生成重复 ID。
   - 证据：`windows_subsystem` supplier 定向测试通过。

3. 非 ccswitch 同 ID 记录不被覆盖
   - 通过标准：同 ID 但非 ccswitch 来源的已有记录仍通过 `uniqueSupplierProfileId` 追加。
   - 证据：`windows_subsystem` supplier 定向测试通过。

4. 安全边界
   - 通过标准：不修改 `assets/inject/claude-chinese-inject.js`，不清空记忆数据库，不输出真实 API Key。
   - 证据：git diff 与测试输出。

## 验证命令

```bash
cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem supplier -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
```

