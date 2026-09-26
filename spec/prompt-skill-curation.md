# 网络安全技能精选迁移

## 背景

用户提供了本地网络安全技能资料库，要求从中选取适合 CCP 系统提示词页的优秀技能迁入仓库 `Prompt/skills`。

## 目标

- 只迁移单个、目录自洽、包含 `SKILL.md` 和明确 Apache-2.0 许可证文件的技能。
- 优先选择与 CCP 工程、供应链、API、日志分析、检测和事件响应直接相关的技能。
- 保留技能目录内的参考资料、脚本和许可证，不复制整个资料库或其仓库历史。
- 让 `scripts/build-prompt-index.mjs` 能生成并校验这些技能的完整清单。

## 非目标

- 不迁移没有明确许可证文件的技能。
- 不迁移整个来源目录、压缩包、无关二进制或来源不明的可执行文件。
- CCP 安装技能时不自动运行其中的脚本。

## 精选清单

- `analyzing-api-gateway-access-logs`
- `analyzing-kubernetes-audit-logs`
- `analyzing-linux-audit-logs-for-intrusion`
- `analyzing-windows-event-logs-in-splunk`
- `building-detection-rules-with-sigma`
- `building-devsecops-pipeline-with-gitlab-ci`
- `triaging-security-incident`
- `triaging-vulnerabilities-with-ssvc-framework`

## 来源与许可证

来源目录：`F:\迅雷下载\网络安全1000+skills\网络安全1000+skills\skills`。

每个精选目录均保留其原始 `LICENSE` 文件，并在 `Prompt/skill-sources.md` 中记录来源、许可证和迁移范围。

## 验证

- 每个目录必须包含 `SKILL.md` 和 `LICENSE`。
- 不得包含符号链接。
- 生成器 `node scripts/build-prompt-index.mjs --check` 必须通过。
- 前端类型检查必须通过。
