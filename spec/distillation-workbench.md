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
4. 项目与标题口径：Claude 与 Codex 的「项目」都取项目文件夹（git 仓库根）的名字；git worktree（含 `.claude/worktrees/<随机名>`，即使 worktree 目录或仓库已删除）一律归并到其主仓库名，不以 worktree 随机目录名作为项目。会话行标题取各客户端的真实会话标题（Claude 的会话标题、Codex 的线程标题），取不到时才回退为项目名。
5. 快速模式：按会话/按项目 + 今天/近 7 天/近 30 天/全部（本地日历日）；“范围内 N 个会话 · 已选 n · 清空”；项目行仅收 git 项目、跨 Agent 合并、最多 3 个来源图标、整体切换；范围外已选自动剔除；不显示模型选择但使用当前模型。
5. 高级模式：素材盒（空态“还没有选择素材 / 在素材库里跨会话勾选对话区间（Hover 消息可设起点/终点）/ 打开素材库”；非空显示会话数/片段数/预估 token、继续添加、清空、会话 chip 最多 8 个）；模型下拉（搜索、按供应商分组、状态点、“离线回退（确定性）”、“+ 新增自有模型配置”）；提示词预设 5 个（精简 120 行 / 附可执行脚本 / 保留踩坑 / 附出处链接 / 中文输出，去重追加“；”）、自定义输入框（Ctrl/⌘+Enter 运行，“{n} 字 · ⌘↵ 运行”）。
6. 素材库弹窗：左栏搜索/来源/时间/项目筛选、按日期粘性分组、行药丸“全选 / on/total / n 条”；右栏消息气泡、点击依次设起止、hover 起点/终点按钮、清除区间、全选整场；每会话一个区间，pickAt/setStart/setEnd 规则同 AITracker；底部区间 chip、统计、清空/取消/确认选择；Esc 关闭。
7. 出产物：能力资产（Skill / 工作流 / Prompt → Skill 库）、记忆资产（画像 / 任务记忆 → 记忆库），hint 与 instruction 文案同 AITracker；前端 promptText = [instruction, 高级模式用户提示词].join("；")。
8. 跑蒸馏：按钮“一键蒸馏 {type}”/“开始蒸馏 {type}”/“蒸馏中…”；提示文案同 AITracker；无可用模型时显示“蒸馏需配置模型。去配置 →”。
9. 后端流水线：校验 selections（OPAQUE id、0≤s≤e、无重复、片段会话必须在 refs 中）；controlledContext 元数据块；片段经 48k/6k、65%/30% `[…内容已压缩…]` 压缩；segmentMarkdown；system = promptForKind（提示词原文移植）+“用户补充要求”；skill/brief/prompt 质检重试最多 2 次（反馈原文）；真实模型非成功即失败；offline 使用确定性占位；候选标题/摘要规则同 AITracker；完成后自动批准入库，画像/任务记忆写入记忆库。
10. 任务：阶段 queued0 / reading-material10 / generating30 / quality-check70 / persisting-candidate90 / syncing-target95 / completed100；持久化 JSON；前端 500ms 轮询，显示进度上限 92%，0 进度时用 elapsed/30s 假进度；活动任务 id 持久化，重进页面继续轮询；保留 CCP 已有取消能力。
11. 历史：运行卡、空态、每页 10 条分页、全选/删除选中（≤100）、行展开 ExpCard（记忆类正文编辑/去记忆库/重新生成；能力类文件树 + 查看/编辑、保存并安装、重新生成）；保存弹窗：名称 slug、实时质检、目标 Agent 复选（默认全选）、目标目录已存在报错不覆盖、写入 `aitracker-origin: distilled`。
12. 模型调用：URL（已带 /chat/completions|/messages|/responses 直用；responses→/responses；anthropic→/vN 后 /messages 否则 /v1/messages；openai→/chat/completions）；请求体（无 temperature、max_tokens 8192、非流式）；头（content-type、Bearer 或 x-api-key、anthropic-version 2023-06-01）；超时 120s；网络错误或 408/425/429/500/502/503/504 重试 2 次间隔 350ms；仅推理无正文判失败；错误码 auth/rate-limited/unavailable/http-client/invalid-response。
13. 视觉：液态玻璃统一风格，颜色只用 `--ops-*` / `--workspace-*` / `hsl(var(--x))`，不使用裸 `var(--primary)` 等 HSL 三元组；窄屏自然堆叠无横向溢出。

## 数据与接口
- Tauri：`load_distillation_workbench`、`run_distillation_workbench`、`query_distillation_task`、`cancel_distillation_task`、`save_distillation_output`、`delete_distillation_candidates`；前端只传供应商 ID/模型 ID，密钥只在 Rust 加载。
- 持久化在 CCP 应用数据目录 `aitracker/`；敏感值不写日志。

## 技术约束
- React/Vite/Tauri/Rust 现有架构；复用 reqwest、SettingsStore、`skill_roots_for_agents`、现有 transcript 读取。
- 遵守 windows_subsystem 文本契约测试。

## 交付范围
规格、验收、React 工作台组件与样式、Rust 流水线/模型客户端/命令、单元测试、构建验证。
