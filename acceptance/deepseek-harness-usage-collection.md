# 验收标准：DeepSeek Harness 会话与用量采集

对应规格：`spec/deepseek-harness-usage-collection.md`

## 通过标准

> 以下各项均已按「执行记录（2026-10-22）」验证通过。

- [ ] 本机实测：DSH 采集到会话与用量（探针输出含 DSH 的会话行数与 token 合计），改造前为 0。
- [ ] DSH 会话标题来自 `session/title`（不是项目名/`unknown`），项目标签为真实工作目录名。
- [ ] `.zstd` glob 对 `dsh` 生效、对其余 source 仍跳过。
- [ ] 多帧拼接的 zstd 文件解压正确：同一文件内多条 `assistant/message` 都能产出记录。
- [ ] 同一会话文件的用量事件 `session_id` 与标题映射的键一致（`dsh_session_identity` 单一来源）。
- [ ] `DISTILLABLE_AGENTS` 含 `dsh`，蒸馏工作台不再过滤 DSH 会话。
- [ ] 现有 Agent（Codex / Claude Code / WorkBuddy）的采集结果不变：全量测试无回归。
- [ ] `cargo fmt --all -- --check`、`cargo test --workspace --no-fail-fast`、
      `npm --prefix apps/claude-codex-pro-manager run check`、`vite:build`、
      `cargo build --release` 全部通过。
- [ ] 解压文本超出 64 MiB 预算时产出告警诊断（不静默截断）。
- [ ] 显示名与标签含 `dsh`：Rust 侧 `agent_display_name` 回落「DeepSeek Harness」，
      前端 Agent 标签与图标（`common.tsx`、`screens.tsx`、`AgentBrandIcon.tsx`）同样识别 `dsh`。
- [ ] `crates/claude-codex-pro-data/src/file_cache.rs` 的 `CACHE_VERSION` ≥ 2
      （解析规则变化后，旧规则产出的载荷不再复用）。

## 必需证据

- `cargo test -p claude-codex-pro-data --lib dsh` 通过，且包含：
  - `dsh_transcript_yields_usage_model_project_and_session`：两帧拼接的会话文件产出用量记录，
    会话 id 来自 `session` 记录、项目取 `cwd` 的目录名、provider/model 取消息 source。
  - `dsh_titles_come_from_the_title_record_or_the_session_directory`：`session/title` 决定标题，
    与用量事件共用同一会话 id；标题缺失时回落首个用户消息。
  - `only_the_dsh_adapters_keep_compressed_transcripts`：只有 `dsh` 的定义保留 `.zstd` glob，
    其余 source 的 `.zstd` 仍被跳过。
- 本机真实数据探针（实现期间执行，结果写入本文件）：读取 `~/.dsh/sessions/` 全部会话文件，
  打印 DSH 会话行数、用量记录数与 token 合计。
- 全量验证命令的原始结果（见「执行记录」）。

## 非目标

- DSH 在供应商/中转、插件与 Skill 安装、启动器等能力面的支持（定义里标记为 unsupported）。
- 工具调用、助手正文的解析与展示。
- 前端页面结构、交互与类型定义的改动。

## 执行记录（2026-10-22）

本机真实数据探针（`crates/claude-codex-pro-data/examples/dsh_probe.rs`，只读 `~/.dsh/sessions/`，
验证后删除）：

| 指标 | 改造前 | 改造后 |
| --- | --- | --- |
| 生效的 dsh 路径 | 0（`.zstd` 全被跳过） | 4 条 glob |
| 用量记录 | 0 | 2450 |
| 解析告警 | — | 0 |
| 会话 | 0 | 25 |
| token 合计 | 0 | 421,797,791 |
| 项目标签 | — | `Claude-Codex-Pro-Tool`、`Project`、`im`（`cwd` 目录名） |
| 标题 | 0 | 29（如 `session-db238160-…` → 「看下本项目：并且遵从以下规」） |

该机 DSH 配置观测到的 model 集合（供参考，不属于判据）：`deepseek-flash`、
`deepseek-v4.1-flash`、`deepseek-v4.1-flash-1017`、`gpt-5.6-sol`；provider 为 `a`、`baidu`、
`deepseek-account`、`gpt`、`tokenplan300`、`topo`。

判据命令与结果：

| 判据 | 命令 | 结果 |
| --- | --- | --- |
| 格式 | `cargo fmt --all -- --check` | 退出码 0 |
| DSH 单测 | `cargo test -p claude-codex-pro-data --lib dsh` | 3 passed / 0 failed / 44 filtered |
| 数据 crate | `cargo test -p claude-codex-pro-data` | 全部目标 `ok`：46 / 4 / 20 / 19 / 17 passed，0 failed |
| 全量回归 | `cargo test --workspace --no-fail-fast` | 退出码 0，43 个目标全部 `ok`、无失败 |
| 前端类型 | `npm --prefix apps/claude-codex-pro-manager run check` | 退出码 0 |
| 前端构建 | `npm --prefix apps/claude-codex-pro-manager run vite:build` | 退出码 0（4.58s） |
| Release | `cargo build --release` | 退出码 0（6m59s）→ `target/release/claude-codex-pro.exe` 28,190,208 字节（本轮构建；迭代中各次构建尺寸一致） |

补充证据（2026-10-23 复核）：

- 截断告警：临时 example 构造解压后约 77 MB 的 DSH 语料（80,000 条 `user/message`）后
  `scan_adapter` 返回 1 条 `Truncated` 诊断（同一文件因末行被截断另产出 1 条解析告警），
  验证后删除该 example；该路径无自动化测试，因为需要超预算的真实语料。
- 显示名与标签：`agent_display_name("dsh")` 返回 `DeepSeek Harness`；前端 diff 只新增 `dsh`
  的图标、名称、图表配色与 `preferred`/`priority` 排序，`npm run check` 与 `vite:build` 通过。
- 缓存版本：`CACHE_VERSION = 2`（`crates/claude-codex-pro-data/src/file_cache.rs`，附说明性注释）。

全量回归过程中的环境性中断（如实记录）：`cargo test --workspace --no-fail-fast` 最初三次都让
`-p claude-codex-pro-core --lib` 目标以

```text
could not execute process `target\debug\deps\claude_codex_pro_core-*.exe` (never executed)
Caused by: 系统找不到指定的文件。 (os error 2)
```

中断——测试二进制在运行前消失，不是断言失败（同一命令单独跑该目标 `569 passed; 0 failed;
16 ignored`；先 `cargo test --workspace --no-run` 预构建也不能消除，故不是构建竞态，而是产物缺失而
fingerprint 仍判 fresh 的缓存态）。处置：`cargo clean -p claude-codex-pro-core` 强制重链后重跑，
第 4 次 `exit 0`、43 个目标全部 `ok`。该命令只清理可再生的构建产物（本次移除 16.8 GiB），
`target/release/claude-codex-pro.exe` 已随后重新构建并核验存在。