# 网络安全技能精选迁移验收

对应规格：`spec/prompt-skill-curation.md`。

## 必须通过

- [x] 精选清单中的 8 个技能目录均存在于 `Prompt/skills`。
- [x] 每个技能均包含 `SKILL.md`、`LICENSE`，且没有符号链接。
- [x] `Prompt/index.json` 包含 8 个新增技能及其全部文件、大小和路径。
- [x] `node scripts/build-prompt-index.mjs --check` 退出码为 0。
- [x] `npm --prefix apps/claude-codex-pro-manager run check` 通过。

## 验证记录

| 检查 | 命令 | 结果 |
| --- | --- | --- |
| 目录完整性 | PowerShell 检查 8 个目录 | 8/8 有 `SKILL.md`、`LICENSE`；0 个符号链接 |
| Prompt 索引 | `node scripts/build-prompt-index.mjs --check` | 23 个提示词、10 个技能、2 个工具包，一致 |
| Prompt library 定向测试 | `cargo test -p claude-codex-pro-core --lib prompt_library -- --nocapture` | 22 passed、0 failed |
| 前端类型检查 | `npm --prefix apps/claude-codex-pro-manager run check` | 通过 |
| 差异空白检查 | `git diff --check` | 通过；仅有既存文件换行提示 |

## 非目标

- 不验证技能脚本对外部系统的运行效果。
- 不把来源资料库完整复制到仓库。
