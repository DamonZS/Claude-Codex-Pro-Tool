# Prompt 内容目录

CCP 的「系统提示词」页从本目录读取提示词、技能和独立工具资源。提示词/技能可部署到已检测的客户端；工具仅下载并安装到 CCP 自己的数据目录，不会自动运行。

远端地址（写死在 core）：

```
https://raw.githubusercontent.com/DamonZS/Claude-Codex-Pro-Tool/main/Prompt/index.json
```

## 每个客户端内容不同

各客户端适配的内容互不相同，因此**投放是逐目标的**：页面上勾选客户端后，为每个客户端
各选一份内容，再一次性下发。提示词用 `targets` 声明适用目标。

## 目录结构

```
Prompt/
  index.json          清单，由脚本生成，勿手改
  prompts/<id>.md     提示词
  skills/<name>/      技能，必须包含 SKILL.md
  tools/<id>/         经筛选的独立源码/二进制文件和 tool.json
```

## 添加提示词

在 `prompts/` 下新建 `.md` 文件，可选 YAML 风格头部：

```markdown
---
title: 显示名称
category: 分类
version: V5
description: 卡片上的一句话说明
targets: codex, zcode
---

正文内容
```

- `targets`：逗号分隔的目标 id，只能取 `codex`、`claude-code`、`deepseek-harness`、
  `zcode`、`workbuddy`、`workbuddy-cn`、`cursor`；写未知目标会导致 CI 校验失败。
  省略 `targets` 表示该条适配全部目标（会出现在每个客户端的可选列表里）。
- `version`：可选的版本标记，用于区分同一客户端的多个版本。
- 没有头部时，文件名作为标题，分类记为「未分类」。

## 添加技能

在 `skills/` 下新建目录，目录内必须有 `SKILL.md`：

```
skills/my-skill/
  SKILL.md           必需，头部可写 name / description
  references/...     可选，子目录会一并分发
```

安装端按清单逐目录复制；卸载只移除清单内记录且内容未变的技能，用户自己放进客户端
`skills/` 的目录不会被触碰。

## 生成清单

```bash
node scripts/build-prompt-index.mjs          # 重新生成 index.json
node scripts/build-prompt-index.mjs --check  # 校验清单与目录一致（CI 使用）
```

改动 `Prompt/` 下的文件后必须重新生成 `index.json`，否则 CI 校验会失败。

## 添加工具

每个工具目录放 `tool.json`、其许可证文件和所需独立文件。`tool.json` 必须记录 ID、版本、简述、GitHub 来源、完整源代码修订、许可证标识/文件和支持平台；生成器为每个分发文件生成 SHA-256 与大小。不得复制完整上游仓库或 Git 历史。只要来源、许可、哈希或文件不完整，索引生成就失败。

CCP 安装工具到应用状态目录 `prompt-tools/<id>/<version>`，不写 PATH、不写客户端配置、不运行工具或包内脚本。卸载前会核对安装文件指纹，检测到外部修改时会保留文件并报告冲突。来源和许可证文件随工具一起下载。

当前首个独立工具是 `tools/codex-instruct/`：只包含上游 `gpt-instruct` 的 Codex CLI、两个官方提示词 ZIP 和 MIT 许可证；源仓库、完整 commit 和文件哈希均在生成的 `index.json` 中记录。
