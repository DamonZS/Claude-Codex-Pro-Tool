# 验收标准：盘古记忆会话采集与自检增强

验证对象：`spec/memory-capture-and-selfcheck-layer.md`

## 验收项

1. 规格与验收文档存在
   - 通过标准：`spec/memory-capture-and-selfcheck-layer.md` 与本文件存在。
   - 证据：文件存在检查或 git diff。

2. 会话采集日志产生
   - 通过标准：注入脚本捕获到用户消息后，会调用本地采集接口并写入采集日志。
   - 通过标准：即使没有生成候选，也能查询到最近采集记录。
   - 证据：定向测试通过；SQLite 查询显示采集表存在并有可插入/读取能力；日志中可看到采集路径。

3. 历史会话采集层可用
   - 通过标准：能从 Codex 本地 SQLite 的 `threads.rollout_path` 定位 rollout 会话文件，并读取真实 `role=user` 消息。
   - 通过标准：历史消息会写入 `memory_captures`；高置信可学习消息通过可审查算法自动进入长期记忆。
   - 通过标准：状态刷新能以有限数量补采历史会话到 `memory_captures`，但普通状态刷新不自动写长期记忆；会话启动摘要刷新、自检修复或显式自动学习路径会对高置信历史消息写入长期记忆。
   - 证据：Rust 定向测试构造临时 Codex SQLite + rollout 文件并通过；DB 计数显示普通刷新只增加采集，自检修复/显式学习增加长期记忆。

4. 采集日志脱敏
   - 通过标准：采集日志不保存完整敏感原文；API key、Bearer token、`sk-` 形态内容被脱敏。
   - 通过标准：采集摘要有长度上限。
   - 证据：Rust 单测或源码断言通过。

5. 学习原因与未触发原因可见
   - 通过标准：采集记录包含 `candidate_triggered`、`candidate_reason`、`skip_reason`。
   - 通过标准：可区分 `no_latest_user_text`、`not_learnable`、`duplicate_recent_memory`、`learn_failed`、`database_failed`。
   - 证据：`cdp_bridge` 或 `bridge_routes` 定向测试通过。

6. DB 计数一致
   - 通过标准：session/status/selfcheck 返回的长期记忆数、待确认候选数来自 `memory_assist.sqlite` 真实计数。
   - 通过标准：不可学习采集日志新增不改变长期记忆数；高置信学习提取会改变长期记忆数；保留的待确认候选仍需确认后才进入长期记忆。
   - 通过标准：相似记忆写入会通过关键词重叠与稳定 SimHash 合并到旧条目，不会无意义重复新增长期记忆。
   - 证据：SQLite 查询与接口测试输出。

7. Runtime 与 manager 状态同步
   - 通过标准：有新鲜 `renderer.memory_runtime` 或采集记录时，manager 能显示分层状态，不再只有泛化 0 条。
   - 通过标准：manager 的“工作区”统计包含长期记忆、待确认候选、采集日志和 Codex 本地会话 workspace；工作区字段能显示 `item_count`、`pending_count`、`capture_count`、`session_count` 和 `latest_capture_at`。
   - 通过标准：自检结果包含 `injection`、`capture`、`candidate`、`database`、`runtime`、`manager` 分层。
   - 证据：manager 定向测试或源码断言通过。

8. 注入摘要缓存可用
   - 通过标准：session 摘要包含已确认记忆、待确认数量和最近采集摘要。
   - 通过标准：本地应用状态目录生成可审查的盘古注入摘要缓存文件，内容来自 `memory_assist.sqlite` 和真实 Codex 会话采集结果。
   - 通过标准：Codex 注入脚本启动时即使当前页面暂无查询文本，也会刷新 session 摘要。
   - 证据：`bridge_routes`、`memory_assist` 或 `cdp_bridge` 定向测试通过；SQLite/缓存文件检查显示摘要存在且脱敏。

9. Workspace 标识稳定
   - 通过标准：注入脚本优先使用真实项目路径或项目 id 作为 workspace。
   - 通过标准：当前 DOM 暂时拿不到项目上下文时，会复用最近一次可信项目上下文，避免立即回落到 `codex:path:*`。
   - 通过标准：`codex:path:*` 仍保留为最后兜底，但不得使用 `document.title`、侧栏标题或当前对话标题生成 workspace。
   - 证据：`cdp_bridge` 定向测试或源码断言通过。

10. 安全边界
   - 通过标准：不修改 Claude 中文注入，不重置或删除用户数据库，记忆实现保持在本项目可审计链路内。
   - 证据：git diff 与命令记录。

## 建议验证命令

```bash
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml memory_assist -- --nocapture
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml --test bridge_routes memory_bridge -- --nocapture
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml --test cdp_bridge memory -- --nocapture
cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem memory -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo build -p claude-codex-pro-manager --manifest-path Cargo.toml
```

## 不在范围内

- 清空、迁移为新库或替换用户现有记忆库。
- 修改 Claude 中文注入脚本。
