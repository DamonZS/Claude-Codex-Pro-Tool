# Codex API 模型选择注入验收标准（已废弃）

> 状态：已废弃。该历史验收文档由 [`acceptance/remove-codex-model-selection-injection.md`](remove-codex-model-selection-injection.md) 取代。

本文件不再定义通过标准，也不应作为 CI、发布或用户验证命令的依据。原有“模型白名单解锁”“模型菜单追加”“CCP 模型增强”及模型请求覆盖验收均已撤销。

当前验收只检查 Codex 模型选择器保持原生行为：CCP 不扫描或修改模型菜单，不注入候选项，不保存或覆盖模型选择，并且管理工具与 Core 不再提供模型白名单解锁设置。请按 [`acceptance/remove-codex-model-selection-injection.md`](remove-codex-model-selection-injection.md) 执行验证。
