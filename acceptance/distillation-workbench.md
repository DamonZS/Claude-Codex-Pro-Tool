# 蒸馏工作台验收标准（AITracker 对齐版）

对应规格：spec/distillation-workbench.md

## 通过标准
- [ ] 统计卡、视图切换、配置卡头、快速/高级模式、素材库弹窗、提示词预设、出产物、跑蒸馏、历史与 ExpCard、保存弹窗的文案与交互和 AITracker 一致（规格 1–8、11）。
- [ ] 视觉为 CCP 液态玻璃风格：统计卡/配置卡/弹窗为半透明玻璃面，颜色只来自 `--ops-*`/`--workspace-*`/`hsl(var(--x))`；蒸馏 CSS 中无裸 `var(--primary|--border|--background|--foreground|--panel)`；760px 以下无横向溢出。
- [ ] Rust 提示词与 AITracker `prompts.ts` 原文一致（单测断言关键句）。
- [ ] 流水线：校验规则、controlledContext、压缩、segmentMarkdown、质检反馈重试、候选标题/摘要规则有单测覆盖。
- [ ] 模型调用：本地 mock HTTP 验证 openai/responses/anthropic 三种协议的 URL、请求体（无 temperature、max_tokens 8192）、请求头；429 后重试成功；仅推理内容判失败。
- [ ] 任务阶段与百分比同规格 10；重新打开页面恢复活动任务轮询；取消进入 cancelled。
- [ ] 完成后候选自动批准；画像/任务记忆写入记忆库；Skill 保存写入所选 Agent 根目录、目录已存在不覆盖、SKILL.md 含 `aitracker-origin: distilled`。
- [ ] 删除选中候选可用（≤100）。
- [ ] 项目口径（规格 4）：`cargo test -p claude-codex-pro-data --lib local_usage` 通过，覆盖存活 worktree、已删除 worktree、仓库也不存在三种情况都归并到主仓库名。
- [ ] 标题口径（规格 4）：实机打开蒸馏工作台，「按会话」每行标题是真实会话标题而不是重复的项目名，「按项目」只出现项目文件夹名（无 `xxx-29f861` 这类 worktree 随机名）。此项需用户实机确认。

## 必需证据
- `npm --prefix apps/claude-codex-pro-manager run check`、`run vite:build` 输出。
- 蒸馏相关 `cargo test` 与 `cargo fmt --check` 输出、`windows_subsystem` 测试输出。
- 浏览器预览截图（深色液态玻璃）。

## 非目标
- 额度、Jarvis 洞察卡、首次引导、知识库数据库、模型 Profile 设置页。
