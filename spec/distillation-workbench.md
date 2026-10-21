# 蒸馏工作台（AITracker 对齐版）

## 背景
用户要求 CCP 蒸馏工作台与 AITracker（`H:\xunlei\aitracker-main\aitracker-main`，`src/modules/distillation/**`、`src/modules/ai-orchestration/**`）保持一致：交互、蒸馏处理逻辑、提示词、质检与模型调用逻辑都对齐；视觉必须使用 CCP 液态玻璃统一风格（`.ops-shell` / `--ops-*` / `--workspace-*` token），不照搬 AITracker 白底样式。用户已声明获得 AITracker 版权方授权；经用户决定，本次不新增 NOTICE 文件。

## 目标
- 前端交互、文案、状态机与 AITracker `DistillationPage` / `DistillConfig` / `MaterialDrawer` / `ExpCard` / `out-types` / `materials` 对齐。
- 后端蒸馏流水线与 AITracker `application/index.ts`、`domain.ts`、`compression.ts`、`prompts.ts`、`qualify.ts` 对齐。
- 模型调用与 AITracker `model-profile.server.ts` 的 URL 拼接、请求体、请求头、重试、错误码、响应解析对齐；模型来源为 CCP 已配置供应商（RelayProfile），外加“离线回退（确定性）”。

## 非目标
- 不移植 AITracker 官方 DeepSeek 额度（quota）、Jarvis InsightCard、首次引导、知识库 SQLite、模型 Profile 设置页；“管理模型”跳转 CCP 供应商页。
- 不改变供应商配置格式、不改无关页面。

## 用户视角
进入“蒸馏工作台”→ 快速模式按会话/项目勾选，或高级模式在素材库跨会话框选消息区间 → 选模型、提示词预设与产物类型 → 跑蒸馏 → 历史中看进度与产物，编辑、重新生成、保存并安装为 Skill（可选目标 Agent），记忆类直接入记忆库。

## 功能要求
1. 统计卡：已选素材（高级=片段数，快速=已选会话数）、素材 Token（Σ turns×900）、蒸馏次数（运行中显示“进行中…”，否则“累计”）、已入库（显示“保存为 Skill”）。
2. 视图切换：蒸馏配置 / 蒸馏历史（带数量徽标），右侧“选素材、配参数、跑蒸馏”。
3. 配置卡头：标题、快速模式/高级配置、状态标签（“自有模型 · 已连接”/“自有模型 · 未配置 Endpoint”）、管理模型。
4. 快速模式：按会话/按项目 + 今天/近 7 天/近 30 天/全部（本地日历日）；“范围内 N 个会话 · 已选 n · 清空”；项目行仅收 git 项目、跨 Agent 合并、最多 3 个来源图标、整体切换；范围外已选自动剔除；不显示模型选择但使用当前模型。
5. 高级模式：素材盒（空态“还没有选择素材 / 在素材库里跨会话勾选对话区间（Hover 消息可设起点/终点）/ 打开素材库”；非空显示会话数/片段数/预估 token、继续添加、清空、会话 chip 最多 8 个）；模型下拉（搜索、按供应商分组、状态点、“离线回退（确定性）”、“+ 新增自有模型配置”）；提示词预设 5 个（精简 120 行 / 附可执行脚本 / 保留踩坑 / 附出处链接 / 中文输出，去重追加“；”）、自定义输入框（Ctrl/⌘+Enter 运行，“{n} 字 · ⌘↵ 运行”）。
6. 素材库弹窗：左栏搜索/来源/时间/项目筛选、按日期粘性分组、行药丸“全选 / on/total / n 条”；右栏消息气泡、点击依次设起止、hover 起点/终点按钮、清除区间、全选整场；每会话一个区间，pickAt/setStart/setEnd 规则同 AITracker；底部区间 chip、统计、清空/取消/确认选择；Esc 关闭。
7. 出产物：能力资产（Skill / 工作流 / Prompt → Skill 库）、记忆资产（画像 / 任务记忆 → 记忆库），hint 与 instruction 文案同 AITracker；Prompt 的 instruction 额外要求「用素材真实内容填满、不要输出空骨架」，保证用户选的就是直接可用的提示词；前端 promptText = [instruction, 高级模式用户提示词].join("；")。
8. 跑蒸馏：按钮“一键蒸馏 {type}”/“开始蒸馏 {type}”/“蒸馏中…”；提示文案同 AITracker；无可用模型时显示“蒸馏需配置模型。去配置 →”。
9. 后端流水线：校验 selections（OPAQUE id、0≤s≤e、无重复、片段会话必须在 refs 中）；controlledContext 元数据块；片段经 48k/6k、65%/30% `[…内容已压缩…]` 压缩；segmentMarkdown；system = promptForKind +“用户补充要求”（skill/brief/persona/memory 为 AITracker 提示词原文移植；prompt 有意改为要求产出「可直接复制使用的系统提示词」，正文必须由素材真实内容填满，占位符最多 2 个）；skill/brief/prompt 质检重试最多 2 次（反馈原文）；真实模型非成功即失败；offline 使用确定性占位；候选标题/摘要规则同 AITracker；完成后自动批准入库，画像/任务记忆写入记忆库。
10. 任务：阶段 queued0 / reading-material10 / generating30 / quality-check70 / persisting-candidate90 / syncing-target95 / completed100；持久化 JSON；前端 500ms 轮询，显示进度上限 92%，0 进度时用 elapsed/30s 假进度；活动任务 id 持久化，重进页面继续轮询；保留 CCP 已有取消能力。
11. 历史：运行卡、空态、每页 10 条分页、全选/删除选中（≤100）、行展开 ExpCard（记忆类正文编辑/去记忆库/重新生成；能力类文件树 + 查看/编辑、保存并安装、重新生成）；保存弹窗：名称 slug、实时质检、目标 Agent 复选（默认全选）、目标目录已存在报错不覆盖、写入 `aitracker-origin: distilled`。
12. 模型调用：URL（已带 /chat/completions|/messages|/responses 直用；responses→/responses；anthropic→/vN 后 /messages 否则 /v1/messages；openai→/chat/completions）；请求体（无 temperature、max_tokens 8192、非流式）；头（content-type、Bearer 或 x-api-key、anthropic-version 2023-06-01）；超时 120s；网络错误或 408/425/429/500/502/503/504 按 1 秒 / 2 秒 / 4 秒指数退避重试 3 次（共 4 次尝试）；HTTP 非 2xx 时把上游响应体折叠空白、替换掉其中的 API Key、截断到 200 字符后附在 `http-error:<status>` 之后，让用户能看到供应商给出的真实原因；仅推理无正文判失败；错误码 auth/rate-limited/unavailable/http-client/invalid-response。
13. 视觉：液态玻璃统一风格，颜色只用 `--ops-*` / `--workspace-*` / `hsl(var(--x))`，不使用裸 `var(--primary)` 等 HSL 三元组；窄屏自然堆叠无横向溢出。
14. 项目与标题口径：Claude 与 Codex 的「项目」都取项目文件夹（git 仓库根）的名字；git worktree（含 `.claude/worktrees/<随机名>`，即使 worktree 目录或仓库已删除）一律归并到其主仓库名，不以 worktree 随机目录名作为项目。会话行标题按以下顺序取第一个可用值：Codex 的 `threads.name`（Codex 侧边栏显示的短会话名）→ WorkBuddy 的 `sessions.custom_title`（优先）或 `sessions.title` → Claude 的会话标题 / Codex 的 `threads.title`（整段首条消息）→ 项目名；取到的标题本身是 URL（含 `://`，如 `codex://threads/<uuid>…`）或纯时间戳（如 `2026-10-18-18-19-23`、`2026/10/18 18:19`）时视为该级无效，继续降级；最终标题折叠空白并截断到 48 字符（超出加 `…`），保证会话列表不出现横向滚动条。
15. 整场读取与输入预算（有意偏离 AITracker 的「整场只传元数据、固定 48k/6k」）：
    - 快速模式选中整场会话或整个项目时，读取会话对话正文交给模型；高级模式仍按框选区间读取。离线模式不读正文。
    - 素材只包含 user / assistant 消息：thinking / reasoning 与 tool 调用及其输出（Claude 读取器渲染为 `Tool call: <name>` 的 tool 消息）一律剔除，避免模型把命令行噪声当成记忆或产物正文。
    - 模型模式下若所选会话一条正文都读不到，任务直接失败并提示「未能读取所选会话的对话正文」，不得只凭元数据生成空泛产物。
    - 模型路径的输入预算由所选供应商的 `contextWindow` 决定：单次调用最多使用窗口的 60%，按 2 字符/token 折算，支持 `1000000`、`200k`、`1m`、`1,000,000`；字段为空或无法解析时退回 48,000 字符，且预算不低于 48,000。单条消息仍受 6,000 字符上限。
    - 素材超过单次预算时按时间顺序分批：每批先提取要点（分批提示词），再用原提示词把全部要点合并成最终产物；质检重试只作用于最终合并。最多 12 批，超出的最新消息不纳入并在任务详情里提示；批与批之间检查取消，进度在 30%~65% 间推进。
    - 请求超时随输入增长：120 秒 + 每 8,000 字符 1 秒，上限 600 秒。
16. 续接链合并：Claude 对话超出上下文后会新建会话文件，首条用户消息是 “This session is being continued from a previous conversation …”。同项目（规格 14 的项目口径）、同非空标题、后一段以续接提示开头、且后一段开始时间紧贴前一段结束时间（最多晚 30 分钟、最多早 5 分钟以容忍时钟误差）的会话视为一条对话：
    - 工作台只显示一行，代表 id 为链内最早且有用量记录的会话；Token/事件/工具调用累加，开始取最早、结束取最晚，模型/项目/状态取最新一段；行副标题显示「· N 段续接」，N 含没有用量记录的成员。
    - 读取转录时链内任一成员都读整条链：按时间顺序拼接，去掉后续段开头的续接提示；超过消息窗口时保留开头 1/5 与结尾 4/5。
    - 无法读取的成员不参与合并，也不会让整条链失败；同一文件含多个会话的来源不参与合并。
17. 工作台加载与刷新：`load_distillation_workbench` 只列可蒸馏 Agent（Codex / Claude / WorkBuddy / Cursor / OpenClaw）的会话；Codex 与 Claude 直接复用请求历史，通用适配器不再重复读同一批会话文件。每次加载只读一次 Claude 会话清单，标题映射与续接链合并共用同一份清单；会话列表与用量快照持久化到应用数据目录（见 spec/session-snapshot-cache.md）：读取先命中内存/磁盘快照，过期后刷新，供应商与候选每次实时读取（编辑供应商后立即生效，蒸馏完成后历史立即出现，不再等待整机重扫）。会话列表最多返回 300 行。前端在拿到首份数据前显示「正在扫描本机会话…」，不显示空列表。

## 数据与接口
- Tauri：`load_distillation_workbench`、`run_distillation_workbench`、`query_distillation_task`、`cancel_distillation_task`、`save_distillation_output`、`delete_distillation_candidates`；前端只传供应商 ID/模型 ID，密钥只在 Rust 加载。
- 持久化在 CCP 应用数据目录 `aitracker/`；敏感值不写日志。

## 技术约束
- React/Vite/Tauri/Rust 现有架构；复用 reqwest、SettingsStore、`skill_roots_for_agents`、现有 transcript 读取。
- 遵守 windows_subsystem 文本契约测试。

## 交付范围
规格、验收、React 工作台组件与样式、Rust 流水线/模型客户端/命令、单元测试、构建验证。
