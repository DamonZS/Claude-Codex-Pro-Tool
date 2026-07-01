# 验收标准：盘古记忆 Runtime 状态同步修复

验证对象：`spec/memory-runtime-status-sync.md`

## 验收项

1. 文档存在
   - 通过标准：本 spec 与验收文档均存在。
   - 证据：git diff 或文件存在检查。

2. manager 不再丢弃真实 runtime 心跳
   - 通过标准：runtime snapshot 反序列化字段有默认值，日志扫描窗口大于 240 行。
   - 证据：源码断言或定向测试通过。

3. `idle` runtime 显示为已注入待会话
   - 通过标准：后端将 `idle` 映射为可用运行态文案；前端 `statusOk` 认可 `idle`。
   - 证据：`windows_subsystem` 或相关定向测试通过。

4. 安全边界
   - 通过标准：不删除、不重置用户记忆数据库；不修改 `assets/inject/claude-chinese-inject.js`。
   - 证据：git diff 与测试输出。

## 验证命令

```bash
cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem memory -- --nocapture
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml --test cdp_bridge memory -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo build -p claude-codex-pro-manager --manifest-path Cargo.toml
```

