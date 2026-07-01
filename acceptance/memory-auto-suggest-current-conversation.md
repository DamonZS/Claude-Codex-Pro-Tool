# 验收标准：盘古记忆当前对话候选修复

验证对象：`spec/memory-auto-suggest-current-conversation.md`

## 验收项

1. 当前状态已被真实检查
   - 通过标准：交付说明包含 `memory_items`、`memory_candidates`、`memory_captures` 的真实计数，以及最近日志中是否存在 `/memory/learn` 或 `/memory/capture`。
   - 证据：SQLite 查询与日志查询输出摘要。

2. 记忆自检指令可触发自动学习
   - 通过标准：注入脚本包含 `memory self-check phrase` 识别路径，覆盖“盘古记忆/这条对话/是否/原因/修复”等关键词组合。
   - 通过标准：自动候选读取用户消息时支持 `codexMemoryUserMessageCandidates` / `nodeOrAncestorLooksLikeCodexUserBubble` 这类受限用户气泡 fallback。
   - 通过标准：自动候选不得使用通用 `[data-testid="conversation-turn"]` 或 `main [class*="user"]` 作为用户角色推断。
   - 证据：`cdp_bridge` 定向测试通过。

3. 未触发学习时可诊断
   - 通过标准：注入脚本包含 `memory_auto_suggest` 诊断事件，并区分 `no_latest_user_text`、`not_learnable`、`duplicate_recent_memory`、`learn_failed`。
   - 证据：源代码断言或定向测试通过。

4. 保持安全边界
   - 通过标准：不删除、不重置用户 `memory_assist.sqlite`；不修改 `assets/inject/claude-chinese-inject.js`。
   - 证据：git diff 与命令记录。

5. 本地调试产物更新
   - 通过标准：`target/debug/claude-codex-pro-manager.exe` 构建成功且时间戳更新。
   - 证据：构建命令与文件时间。

## 建议验证命令

```bash
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml --test cdp_bridge memory -- --nocapture
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml --test bridge_routes memory_bridge -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo build -p claude-codex-pro-manager --manifest-path Cargo.toml
```
