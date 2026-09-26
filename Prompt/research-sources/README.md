# 研究资源迁移清单

这里保存从 `F:\项目代码备份` 研究副本中筛选出的原始资料，不属于默认提示词、Skill 或工具安装清单。
CCP 不会自动加载、投放或执行本目录内容；原始仓库仍保留在 `F:\项目代码备份`。

## 已迁移文件

- `leila/assets/`：Leila 1.0.7 的完整离线提示词、AC Skill、身份 Skill、agent 元数据、引用资料和 manifest。
- `deepseek-v4-pro-unrestricted/prompts/`：三个 prompt-bank 提示词版本及 MIT 许可证。
- `jailbreak-prompts/`：各模型研究样本、README 和 MIT 许可证。
- `gpt-instruct/prompts/`：gpt-5.6-sol-v45、gpt-6-astra-v1 两个发布提示词及 MIT 许可证。
- `pentestdb/`：README、LICENSE、NOTICE 元数据；攻击载荷、WebShell、凭证库和 exploit 未复制。
- `Pentest-tools/`：仅 README；仓库无统一再分发许可证，二进制和驱动未复制。
- `Domain-penetration_one-stop/`：仅 README；原始手册和 PDF 未复制，仓库许可证状态待核验。
- `Permeable/`：仅 README；原始漏洞、WebShell、二进制和压缩包未复制。

## 可安装资源

- `../skills/leila-identity/` 是唯一从 Leila 资源直接纳入正式 Skill 清单的内容。
- `../tools/deepseek-prompt-evaluator/` 是 DeepSeek 评测脚本的显式安装包，含来源、修订、许可证和文件指纹；CCP 安装后不运行其中脚本。
- AC/unrestricted、jailbreak 和攻击资料只保留在本研究目录，不进入 `Prompt/index.json` 的默认投放列表。

## 来源与边界

所有来源仓库的完整副本、Git 历史、运行输出和用户级配置均留在 `F:\项目代码备份`。没有根许可证或无法确认独立二进制再分发权的内容只记录元数据，不进入可安装清单。
