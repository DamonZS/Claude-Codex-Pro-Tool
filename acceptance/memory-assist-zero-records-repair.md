# 验收标准：盘古记忆 0 条记录修复

验证对象：`spec/memory-assist-zero-records-repair.md`

## 验收项

1. 规格与验收文档存在
   - 通过标准：本文件与 `spec/memory-assist-zero-records-repair.md` 均存在。
   - 证据：文件存在检查。

2. workspace 不再来自标题
   - 通过标准：注入脚本的 `codexMemoryWorkspace()` 不包含 `document.title`、`[data-thread-title]`，并包含 `codex:thread:` 或 `codex:path:` fallback。
   - 证据：`cdp_bridge` 定向测试通过。

3. 自动候选覆盖真实项目指令
   - 通过标准：注入脚本包含 `codexMemoryLooksLearnableText`，能识别项目约束、偏好、修复要求、UI/工作流要求等真实用户消息。
   - 通过标准：过短消息、寒暄、纯标题不会生成候选。
   - 证据：`cdp_bridge` 定向测试通过。

4. 候选计数来自后端
   - 通过标准：候选创建成功后不只做本地 `+1`，而是用后端返回值或重新加载 session 同步 `pendingCandidates`。
   - 证据：源码检查与定向测试通过。

5. 记忆数据不被破坏
   - 通过标准：修复过程不删除、不重置用户 `memory_assist.sqlite`。
   - 证据：无删除数据库命令，构建与测试不依赖用户数据库清空。

6. 本地 debug 产物更新
   - 通过标准：`target/debug/claude-codex-pro.exe` 与 `target/debug/claude-codex-pro-manager.exe` 构建成功，时间戳更新。
   - 证据：构建命令输出和文件时间。

## 验证命令

优先运行：

```bash
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml --test cdp_bridge memory -- --nocapture
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml --test bridge_routes memory_bridge -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo build -p claude-codex-pro-launcher --manifest-path Cargo.toml
cargo build -p claude-codex-pro-manager --manifest-path Cargo.toml
```

## 不在范围内

- 自动批准候选进入长期记忆。
- 清空、迁移或替换用户现有记忆库。
- 修改 Claude 中文注入脚本。
