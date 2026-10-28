# 验收标准：蒸馏子代理、可安装技能包与卡片库

对应规格：`spec/distillation-agent-and-card-library.md`

## 通过标准

1. 蒸馏子代理接受 `会话蒸馏 / 技能蒸馏 / 踩坑蒸馏` 三类意图，返回 `kind`；三类意图各有至少 1 个测试用例覆盖。
2. 子代理返回 `skipped: true` 与 `reason` 时不产生产物；该分支有测试覆盖。
3. `run_distillation_agent` 对空指令与空会话列表返回可读错误（错误码稳定），有测试覆盖。
4. 技能包结构正确：`<name>/SKILL.md` 存在，`SKILL.md` 含 `name` 与 `description`，且描述非空骨架（不含 `TODO`/`占位` 等占位文本）。
5. 一键安装：目标不存在时安装成功并在插件中心/技能目录可见；目标已存在时返回冲突结果而非静默覆盖；两条路径各有测试。
6. 卡片库存储为单一 JSON，字段齐全（`id/kind/title/body/tags/source/createdAt/updatedAt`）；读写往返测试通过。
7. 模型返回非法 `kind` 时降级为 `experience` 且标签为空，并记录诊断；有测试覆盖。
8. 标签处理：去重、限长、限数量；有测试覆盖。
9. 查询支持 `kind`、`tag`、关键词过滤与分页；有测试覆盖。
10. `acceptance` 内所有命令均实跑并在「执行记录」登记（命令 + 结果）。
11. 前端卡片库页面可加载、过滤、打开编辑弹窗（编辑可选）、导出单卡；`tsc` 与 `vite:build` 通过。
12. 不编辑也能完成创建、检索、导出（编辑不是必经路径）。
13. P2 建议：候选只落入待确认列表，不自动写库；接受/忽略两条路径各有测试。
14. 全量验证通过：`cargo fmt --all -- --check`、`cargo test --workspace --no-fail-fast`、`npm --prefix apps/claude-codex-pro-manager run check`、`npm --prefix apps/claude-codex-pro-manager run vite:build`，并重建 `target/release/claude-codex-pro.exe`。

## 必需验证方式

- Rust：`cargo test -p claude-codex-pro-core <对应模块/测试名> -- --nocapture`，以及 `cargo test --workspace --no-fail-fast`。
- 前端：`npm --prefix apps/claude-codex-pro-manager run check` 与 `run vite:build`。
- 真实数据抽样：在本机 CCP 数据目录上跑一次卡片库读写与技能包安装（可安装到临时目录），记录前后文件清单。
- 证据：命令、原始输出摘要、产物路径（含 `SKILL.md` 片段）。

## 已知取舍

- 技能包只落本机，不提供云端分发；分享由用户自行导出。
- 卡片建议只入队，不自动写库（避免误记）。
- 模型判定类型/标签失败时降级，不阻塞卡片创建。

## 非目标

- 不做云端同步、账号体系、市场审核。
- 不自动修改用户 Codex / Claude 既有配置与技能。
- 不覆盖既有「一键蒸馏」行为（新能力是增量入口）。