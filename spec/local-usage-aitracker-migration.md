# AITRACKER 本地采集器与解析契约迁移

## 背景

CCP 概览页已经可以读取 Codex、Claude Code 和 CCP 代理记录，但采集逻辑仍集中在单一请求历史读取器中，无法复用 AITRACKER 的工具注册、字段映射、会话匿名化、统一事件和聚合模型。后续需要覆盖更多 Agent 工具、会话管理和使用量分析，因此先迁移采集层的稳定契约。

## 目标

- 在 `claude-codex-pro-data` 中建立与 AITRACKER 等价的本地用量数据契约。
- 支持工具注册、JSON/JSONL/SQLite 候选、字段路径映射、统一 Token 事件、匿名 session ID、按 Git 根归一的项目身份、聚合快照和诊断。
- 保持只读扫描、文件大小/记录数/目录深度上限，不读取或保存 prompt、消息正文、API key 或完整本地路径。
- 保持现有概览页与 CCP 外层 UI 不变，并让现有 Codex/Claude/代理记录可以转换到统一事件模型。
- 为后续新增 Agent 工具保留注册入口，不在本阶段硬编码所有 AITRACKER 工具的专用解析器。

## 非目标

- 不迁移 AITRACKER 的 Next.js、Node.js、React 页面或远端服务。
- 不在本阶段添加所有第三方 Agent 的专用 schema；专用 schema 通过后续 adapter 逐个接入。
- 不改变 CCP 外层导航、顶栏、液态玻璃样式或现有会话业务接口。

## 技术要求

1. `local_usage` 模块提供 `LocalUsageEvent`、`LocalUsageSnapshot`、`UsageAdapter`、`UsageFieldMapping`、`UsagePath` 和诊断类型。
2. session ID 使用域隔离的 SHA-256 截断值，结构化 session 与相对文件身份均不得导出原始值。
3. 字段映射支持点路径、JSON/JSONL 根数组或单对象，以及 SQLite 查询列；Token 总量优先使用组件合计，组件缺失时使用映射的 total 字段。
4. scanner 只读取最近窗口内的候选文件，并执行文件字节、文件数、目录项、SQLite 行数和返回事件数量上限；单行损坏只产生诊断并继续扫描。SQLite 仅允许只读单条 SELECT/WITH 查询。
5. aggregate 输出 real/empty 模式、总量、按来源/模型/项目/日期 breakdown、详情和最近事件。
6. 代理 `RequestRecord` 与本地 scanner 事件在转换时保留 source、agent、provider、model、session、project、status、duration 和 token 字段。

## 验收

- Rust 单元测试覆盖 session ID 稳定性/隐私、字段映射、JSONL 坏行跳过、Token 合计、重复事件去重、项目路径末段和空快照。
- `cargo test -p claude-codex-pro-data --lib local_usage` 与既有 `request_history` 测试通过。
- `npm --prefix apps/claude-codex-pro-manager run check` 通过，概览页仍使用现有 `RequestTimelineResult`，无外层 UI 变更。
- `git diff --check` 通过。
