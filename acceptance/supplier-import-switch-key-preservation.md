# 验收标准：供应商导入与切换 Key 保留修复

验证对象：`spec/supplier-import-switch-key-preservation.md`

## 验收项

1. 规格与验收文档存在
   - 通过标准：`spec/supplier-import-switch-key-preservation.md` 和本文档均存在。
   - 证据：文件存在检查或 git diff。

2. 前端能识别非 `apiKey` 字段中的 key
   - 通过标准：`App.tsx` 中存在统一的供应商 key 解析函数，可从 `authContents` 和 `configContents` 解析 key。
   - 通过标准：`normalizeSupplierProfile` 不再只使用 `profile.apiKey ?? ""`。
   - 证据：定向测试或源码断言测试通过。

3. 点击使用不再误拦截已带 key 的导入供应商
   - 通过标准：`switchCodexRelayProfile` 和 `saveAndSwitchDraft` 使用统一 key 判断函数，而不是直接检查 `profile.apiKey.trim()`。
   - 证据：定向测试或源码断言测试通过。

4. 重新生成配置不丢 key
   - 通过标准：`withSupplierGeneratedFiles` 在生成 `authContents` 前保留或恢复已有 key。
   - 通过标准：导入 ID 冲突时重新生成 provider id 对应的配置内容。
   - 证据：定向测试或源码断言测试通过。

5. ccswitch 导入支持 config token
   - 通过标准：后端导入逻辑可从 `experimental_bearer_token` 提取 key。
   - 证据：Rust 单元测试通过。

6. 安全边界
   - 通过标准：不修改 `assets/inject/claude-chinese-inject.js`，不清空用户记忆数据库，不把真实 key 写入日志。
   - 证据：git diff 和测试命令。

## 验证命令

优先运行：

```bash
cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem supplier -- --nocapture
cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml ccswitch -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo build -p claude-codex-pro-manager --manifest-path Cargo.toml
```

## 不在范围内

- 不验证第三方真实 API 的连通性。
- 不改变供应商页面整体视觉设计。
- 不处理 Claude 前端注入问题。
