# 盘古记忆独立页、能力可视化与增量采集验收标准

对应规格：`spec/pangu-memory-page-incremental-context-capture.md`

## 验收项

1. 文档
   - `spec/pangu-memory-page-incremental-context-capture.md` 存在且为 UTF-8 中文。
   - `acceptance/pangu-memory-page-incremental-context-capture.md` 存在且为 UTF-8 中文。

2. 导航与页面结构
   - 左侧导航出现 `盘古记忆`。
   - 位置在 `会话管理` 下方、`维护` 上方。
   - `memoryAssist` 旧入口能归一到 `memory` 路由。

3. 概览页
   - 盘古记忆卡只保留开关、运行状态、Codex 注入、对话监控和火焰特效。
   - 不显示经验教训、采集记录、工作区、数据库、最近备份、查看/编辑、提炼、自检等入口。
   - `MemoryActivityWave` 组件和火焰特效不被修改。

4. 会话管理页
   - 不再渲染 `MemoryAssistPanel`。
   - 保留历史会话修复、Codex 会话管理、Claude 会话诊断。

5. 独立盘古记忆页
   - 顶部展示三层链路状态：历史会话采集层、注入实时监听层、核心算法裁判层。
   - 展示增量采集状态，并使用 `采集进度` 文案。
   - 展示模块 C：单个 Markdown 会话经验教训注入手册、目录、编辑/保存/复制/重新提炼/来源入口。
   - 展示模块 B：active、archived、常驻、strength、retention、归档/恢复。
   - 展示模块 D：MCP 开关、共享数据库、注册状态和四个工具能力。
   - 来源条目默认折叠，可展开搜索、显示归档、编辑、删除、归档/恢复、导入导出。

6. 采集层
   - Codex rollout 可解析 `user`、`assistant`、`tool`、`system`、`developer` 等 message role。
   - Claude `audit.jsonl`、`local_*.json`、`.claude/sessions/*.json` 等可解析 JSON/JSONL 源会进入增量扫描。
   - 后端存在真实 `memory_capture_progress` 采集进度表或等价结构，且包含最近扫描新增与跳过未变化统计。
   - 第二次扫描未变化来源时不会重复写入 capture，并能在状态中体现跳过未变化来源。
   - 追加 Claude/Codex 会话内容后，只新增采集追加上下文，并更新最近扫描新增上下文。
   - 短文本、普通对话和 agent 输出不会在采集层被丢弃。
   - 采集层不决定长期记忆，不决定注入手册；只写采集证据。

7. 验证命令
   - `npm --prefix apps/claude-codex-pro-manager run check` 通过。
   - `npm --prefix apps/claude-codex-pro-manager run vite:build` 通过。
   - `cargo test -p claude-codex-pro-core --manifest-path Cargo.toml memory_assist -- --nocapture` 通过。
   - `cargo build -p claude-codex-pro-manager --manifest-path Cargo.toml` 通过。
   - 构建后确认 `D:\Project\Claude-Codex-Pro-Tool\target\debug\claude-codex-pro-manager.exe` 更新时间变化。

## 非目标

- 不要求本次实现完整 Claude 官方缓存读取。
- 不要求重置采集进度或数据库。
- 不要求修改 Claude 中文注入。
- 不要求重做概览页对话监控火焰动画。
