# DeepSeek Harness 会话与用量采集

## 背景

CCP 的「会话」页、「蒸馏工作台」与概览用量都来自本机 AITracker 式扫描：按
`assets/aitracker/definitions/*.tool.json` 的路径与 glob 读取各 Agent 的会话日志。DeepSeek
Harness（下称 DSH）的记录一直存在于 `~/.dsh/sessions/`，定义文件 `dsh.tool.json` 也已随包
发布，但实测**一条 DSH 会话与用量都采集不到**。源码取证到三条原因：

1. 定义解析直接跳过 `.zstd` 结尾的 glob（`crates/claude-codex-pro-data/src/local_usage.rs` 的 `definition_adapters`），
   而 DSH 的 4 条 glob 中 2 条是 `.jsonl.zstd`（另 2 条 `.jsonl` 在本机不存在），于是没有任何
   文件进入扫描。
2. 原生读取派发只覆盖 `codex` / `every-code` / `claude-code`
   （`crates/claude-codex-pro-data/src/request_history.rs` 的 `read_native_usage_file`，扫描侧同一门槛见
   `local_usage.rs` 的 `scan_adapter`），`dsh.tool.json` 又没有 `mapping`，通用字段抽取同样产不出记录。
3. 蒸馏工作台的可选 Agent 白名单 `DISTILLABLE_AGENTS`
   （`apps/claude-codex-pro-manager/src-tauri/src/commands.rs`）不含 `dsh`。

DSH 的日志格式与其它 Agent 都不同：每个会话一个文件 `session.v{3,4}.jsonl.zstd`，是**多帧拼接**
的 zstd 流（本机 29 个文件，最大 7.6 MB 压缩、内含 4670 个 zstd 帧），解压后是逐行的 JSON 信封。
实测（2026-10-22 探针）各记录形状如下：

- `session`：`{"type":"session","version":4,"id":"session-<uuid>","createdAt":<ms>,"cwd":"<工作目录>",
  "isSeeded":false,"delegationDepth":0,"agentPreset":"standard"}` —— **id 与 cwd 在信封顶层，没有
  `data` 字段**；会话目录名同样是该 id（旧会话为裸 uuid）。
- `session/title`：`{"type":"session/title","seq":<n>,"time":<ms>,"data":{"title":"…","messageSeqs":[…],
  "source":{"kind":"fallback"}}}`。
- `assistant/message`：`{"type":"assistant/message","seq":<n>,"time":<ms>,"id":"…","data":{"message":
  {"role":"assistant","content":[…],"source":{"kind":"model","provider":"…","model":"…"}},
  "usage":{"inputTokens":…,"outputTokens":…,"cacheReadTokens":…,"cacheWriteTokens":…,"totalTokens":…}}}`。
- `user/message`：`{"type":"user/message","data":{"content":[{"type":"text","text":"…"}]}}`。

## 目标

- 采集 DSH 的**会话与用量**：会话页列出 DSH 会话（标题、项目、模型、token），概览用量统计含 DSH。
- 会话标题取 DSH 自己的 `session/title`，缺失时回落首个用户消息，再缺才回落项目名。
- 蒸馏工作台能把 DSH 会话当作材料。
- zstd 解码使用官方 `zstd` crate（用户 2026-10-22 选型），其余架构不变。

## 非目标

- 不让 DSH 出现在供应商/中转、插件与 Skill 安装、启动器等能力面：定义里
  `capabilities.agents/skills/market/security` 均为 `unsupported`，本次不改。
- 不解析工具调用与助手正文，只取会话、用量、标题。
- 不改 `dsh.tool.json` 的路径定义，也不改其它 Agent 的解析行为。
- 不为 DSH 新增加速索引、数据库或持久化格式。

## 用户视角

1. 打开「会话」页能看到 DeepSeek Harness 的会话：标题是会话自己的标题而非项目名，项目在副标题，
   token 有真实数值。
2. 概览的用量统计与 Agent 排行包含 DSH。
3. 蒸馏工作台的材料列表里可以选到 DSH 会话。

## 功能要求

1. 定义解析：`.zstd` glob 不再被一律跳过，只在有原生压缩读取支持的 source 上保留（本次为 `dsh`）。
2. 新增 `dsh` 原生读取器：
   - 解压支持多帧拼接的 zstd 流；解压文本上限 64 MiB（超出截断并告警），压缩文件超过 `MAX_FILE_BYTES`
     时按既有规则跳过。
   - `session` 记录 → 会话 id 与项目（`cwd`）。
   - `session/title` 记录 → 标题。
   - `assistant/message` 记录 → 一条用量记录：provider/model 取 `data.message.source`，用量取
     `data.usage`（`inputTokens` / `outputTokens` / `cacheReadTokens` / `cacheWriteTokens` /
     `totalTokens`），时间取信封 `time`（毫秒）；同一消息 id 重复出现按快照合并（沿用 `merge_snapshot`）。
   - 会话 id 走统一派生 `dsh_session_identity`：有 `session.id` 时用 structured 族哈希，否则用文件路径族哈希。
3. 标题映射：新增 `dsh_session_titles(home)`，按定义里的 DSH 路径枚举文件，只解压文件头部（1 MiB 上限）
   取 `session/title`；标题为空时取首个 `user/message` 文本，并沿用既有 `clean_session_title` 的
   空白折叠与长度截断。接线进 `distillation_session_titles()`，键为 `dsh:<会话 id>`。
4. 白名单：`DISTILLABLE_AGENTS` 增加 `dsh`。
5. 显示名：Rust 侧 `agent_display_name` 与前端 Agent 标签/图标补充 `dsh`（图标复用既有 `deepseek.svg`）。
6. 缓存：沿用按 `(path, mtime, size)` 的文件解析缓存；解析规则变化时递增 `file_cache::CACHE_VERSION`。

## 数据与接口

- 新增依赖：`crates/claude-codex-pro-data/Cargo.toml` 增加 `zstd`，仅数据 crate 使用。
- 数据来源只读：`~/.dsh/sessions/` 下的 `session.v*.jsonl.zstd`；不写入 DSH 目录。
- 不改 Tauri 命令签名、前端类型与页面结构。

## 技术约束

- 解析放 `crates/claude-codex-pro-data`，命令接线放 `apps/claude-codex-pro-manager/src-tauri`。
- 除 `zstd` 外不新增依赖，不新增数据库。
- 扫描保持有界：文件数、单文件大小、事件数沿用现有上限。
- 默认 Release 可执行文件仍需可构建。

## 交付范围

规格与验收；`crates/claude-codex-pro-data`（依赖、定义解析、`dsh` 原生读取器、标题映射、单测）；
`commands.rs`（白名单与标题接线）；前端 Agent 标签/图标；本机实测数据与全量验证。

## 变更记录

- 2026-10-22：首版。新增 `zstd` 依赖与 `dsh` 原生读取器、`.zstd` 门按 source 放开、标题映射、白名单
  与前端标签补充。
- 2026-10-22（探针实测修正）：`session` 记录的 `id`/`cwd` 位于信封顶层而非 `data` 内；按 `data` 取值
  时全部 DSH 会话的 `session_id` 回落成空、项目名错误地回落为显示名，已在解析器与单测夹具中改正。
- 2026-10-22：验证用的临时探针 `crates/claude-codex-pro-data/examples/dsh_probe.rs` 用后删除，
  实测数字与判据命令记录在 `acceptance/deepseek-harness-usage-collection.md`。