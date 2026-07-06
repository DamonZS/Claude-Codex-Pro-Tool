# 验收标准：盘古记忆通用 Agent 大脑增强

验证对象：`spec/memory-universal-agent-brain.md`

本验收覆盖四项能力，按阶段推进。每个阶段可独立验收、独立交付，不得因后续阶段未完成而阻塞前一阶段的验收结论。

## 通用前置

1. 规格与验收文档存在
   - 通过标准：`spec/memory-universal-agent-brain.md` 与本文件存在。
   - 证据：文件存在检查或 git diff。

2. 不破坏现有链路
   - 通过标准：现有 `memory_assist.sqlite` 可自动迁移，不清空 `memory_items` / `memory_candidates` / `memory_captures`。
   - 通过标准：现有采集、注入、自检、导入导出链路仍可用。
   - 通过标准：`cargo test --workspace` 全绿；`npm --prefix apps/claude-codex-pro-manager run check` 通过。
   - 证据：迁移前后 DB 计数对比；测试输出。

3. 不引入 Python / 外部进程依赖到核心检索与压缩路径
   - 通过标准：语义检索、遗忘曲线、分层、压缩的核心逻辑全部在 Rust 内实现；不依赖新的外部可执行程序或网络服务。
   - 通过标准：向量存储使用 SQLite（BLOB 或 sqlite 向量扩展），不新增 Python 运行时。
   - 证据：`Cargo.toml` diff；源码审阅。

## 阶段 1：语义检索（FTS5 + 向量混合）

4. FTS5 全文索引可用
   - 通过标准：`memory_items` 拥有对应的 FTS5 虚拟表（或等价全文索引），学习/更新/删除记忆时索引同步维护。
   - 通过标准：旧库首次启动时能对存量 `memory_items` 回填全文索引。
   - 证据：Rust 定向测试插入记忆后用 FTS5 MATCH 查询命中；迁移测试验证回填。

5. 向量嵌入存储与检索
   - 通过标准：`memory_items` 可存储 embedding 向量（BLOB 或向量扩展列）。
   - 通过标准：检索时计算查询向量与候选向量的余弦相似度并纳入排序。
   - 通过标准：embedding 生成失败或不可用时，检索自动降级为 FTS5 + 关键词，不报错、不丢结果。
   - 证据：Rust 定向测试：构造带 embedding 的记忆，语义近义查询能命中关键词不重叠的条目；embedding 缺失时降级路径测试通过。

6. 混合排序融合
   - 通过标准：最终排序融合三路信号——向量余弦、FTS5/关键词得分、既有 `score_item` 元数据加成（category/tag/access/global）。
   - 通过标准：融合策略有明确、可测的权重或 RRF 规则，不是随意相加。
   - 通过标准：跨语言近义（如查“发布”命中“push/deploy”，查“send”命中“publish”）在有 embedding 时可命中。
   - 证据：Rust 定向测试覆盖融合排序；近义命中用例通过。

7. 检索性能不劣化
   - 通过标准：语义检索路径不把状态轮询变成全表 O(N²)；沿用或增强现有 fingerprint 防抖。
   - 证据：源码审阅说明；必要时基准或计数断言。

## 阶段 2：遗忘曲线 + 分层

分层模型：两层 `tier`——`active`（活跃，进入注入摘要）与 `archived`（归档，软标记、可恢复、不进注入）。详见 `docs/adr/0001-pangu-memory-decay-and-tiering.md`。

8. 两层分层字段
   - 通过标准：`memory_items` 新增 `tier` 字段（默认 `active`），v3→v4 迁移把现有行填为 `active`，不丢数据、不清表。
   - 通过标准：`tier` 是可查询、可过滤的状态，不新建多表。
   - 证据：schema/字段 diff；v3→v4 迁移测试保留全部旧行并填默认 `active`。

9. Ebbinghaus 指数衰减与访问增强
   - 通过标准：retention = 初始强度 × exp(−Δt/τ)，Δt 自 `last_accessed_at` 起算；命中检索时重置计时并叠加强度。
   - 通过标准：默认 τ≈30 天（半衰期），retention 低于归档阈值（≈0.12，约 90 天未命中）触发归档。
   - 通过标准：衰减为读时懒惰计算，复用现有 fingerprint 防抖，不新增后台定时线程。
   - 通过标准：衰减不物理删除任何记忆，只改 `tier` 与排序权重。
   - 证据：Rust 定向测试：老旧未命中记忆 retention 下降并归档；命中后 retention 回升并回到 active；读时计算不触发全表 O(N²)。

10. 豁免规则
   - 通过标准：`source=manual`、`category=safety-rule`、`category=project-rule` 的记忆豁免衰减，永久留在 `active`，除非用户显式手动归档。
   - 证据：Rust 测试：豁免类记忆在超过归档阈值的时间后仍为 `active`。

11. 归档可见性与检索
   - 通过标准：注入路径（会话摘要 / inject 缓存）只取 `tier=active`；归档记忆不进注入。
   - 通过标准：管理器搜索默认只搜 active，可选开关“含归档”后才纳入 archived。
   - 证据：Rust 测试：注入结果不含 archived；搜索开关控制 archived 可见性。

12. 手动归档 / 恢复对称能力
   - 通过标准：用户可手动把一条 active 记忆归档（即使未达阈值），也可手动恢复一条 archived 记忆到 active（恢复后重置计时）。
   - 通过标准：自动归档、手动归档、手动恢复都写入 `memory_events` 审计。
   - 证据：Rust 测试：手动归档/恢复改变 tier 并留审计事件。

13. 前端分层与强度界面
   - 通过标准：记忆列表默认只显示 active；提供“显示归档”开关，归档项置灰 + “已归档”标签 + “恢复”按钮。
   - 通过标准：active 记忆卡片显示 retention 细进度条 + 悬停数值；豁免记忆显示为满并标“常驻”。
   - 通过标准：active 记忆卡片提供“归档”按钮。
   - 证据：manager 定向测试或源码断言；真机截图证明分层展示、强度条、归档/恢复入口存在。

## 阶段 3：LLM 压缩整合

14. 增量分层整合替代全表删表
    - 通过标准：整合不再一次性 `DELETE` 全部 `memory_items` 再写 1 条 bullet。
    - 通过标准：整合按 workspace / 层级增量进行，保留原始记忆（可归档），生成的摘要是新增的高层记忆而非唯一幸存者。
    - 通过标准：整合结果的合成条目 id 稳定（同一 workspace + 同一层的整合可稳定 upsert，不因 `now_nanos()`/pid 抖动）。
    - 证据：Rust 定向测试：整合前后原始记忆仍可查；重复整合不产生 id 抖动、不丢历史。

15. LLM 摘要式压缩（可降级）
    - 通过标准：整合支持调用 LLM 生成摘要（复用本项目已有的模型调用通道 / provider 配置）。
    - 通过标准：LLM 不可用（无 key、离线、超时）时，自动降级为现有规则式 `lesson_sentence_score` 抽句，不报错、不阻塞。
    - 通过标准：压缩输出仍走脱敏，不落敏感原文。
    - 证据：Rust 定向测试：LLM 通道 mock 命中；降级路径命中；脱敏断言通过。

16. 审计可回放
    - 通过标准：整合写 `memory_events`（如 `items_compacted`），记录来源条目与产出条目，使“记忆如何演化”可回放。
    - 证据：Rust 测试查询事件流验证。

## 阶段 4：MCP 跨 Agent 共享

17. MCP server 暴露盘古记忆
    - 通过标准：提供一个 MCP server，暴露至少 `memory_search` / `memory_learn` / `memory_recent`（或等价）工具。
    - 通过标准：MCP 读写复用同一套 `memory_assist.sqlite` 与门控，不旁路脱敏与写开关。
    - 证据：MCP server 定向测试或真实 MCP 客户端调用记录；调用后 DB 计数变化。

18. 跨 agent workspace 语义
    - 通过标准：workspace 键不再绑死 Codex；提供统一 workspace 语义，使 Claude / Codex / 自定义 CLI 能映射到同一或可隔离的记忆空间。
    - 通过标准：现有 `codex:repo:` / `codex:thread:` 等键仍兼容。
    - 证据：Rust 测试验证新 workspace 解析与旧键兼容。

19. Claude 侧可接入
    - 通过标准：解除 `claude_injected` 恒为 false 的限制，或通过 MCP 让 Claude 侧能读写盘古记忆（二选一，视架构决定）。
    - 通过标准：不修改 `assets/inject/claude-chinese-inject.js` 的既有中文注入职责（记忆接入若走注入需独立通道）。
    - 证据：源码审阅 + 测试。

## 证据与非目标

- 每阶段必须提供真实验证：`cargo test` 定向用例输出、必要的 DB 计数、构建结果。
- 允许分阶段交付：阶段 1 完成即可单独验收通过，不要求四阶段一次性完成。
- 非目标：不做云端多租户、不做分布式记忆同步、不引入外部向量数据库服务、不改动与记忆无关的 UI。
- 安全底线：全程不落 API key / Bearer token / `sk-` 原文；不删除或重置用户现有记忆库。
