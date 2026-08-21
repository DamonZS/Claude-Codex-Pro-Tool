# Codex API 模型选择注入（已废弃）

> 状态：已废弃。该历史规格由 [`spec/remove-codex-model-selection-injection.md`](remove-codex-model-selection-injection.md) 取代。

## 说明

Codex 现在已经能够由自身正常渲染和管理模型选择器。CCP 过去用于解锁模型白名单、追加自定义模型和覆盖模型请求的注入链已停止维护并从当前目标中移除。

本文件仅保留为历史记录，不能作为当前实现、产品能力、测试或验收依据。当前实现不得通过 DOM 扫描、状态补丁、模型目录响应补丁、请求覆盖或任何其他前端增强改变 Codex 的模型列表和选择结果。

## 当前依据

- 当前目标和禁止事项：[`spec/remove-codex-model-selection-injection.md`](remove-codex-model-selection-injection.md)
- 当前验收标准：[`acceptance/remove-codex-model-selection-injection.md`](../acceptance/remove-codex-model-selection-injection.md)
- 供应商模型目录字段仍可用于管理工具的配置与路由，但不代表 CCP 会修改 Codex 原生模型菜单。

## 历史交付物处理

旧版模型选择注入契约、模型菜单候选测试和“CCP 模型增强”界面均不再是交付范围。后续维护不得恢复这些入口；如需修改 Codex 原生模型选择行为，必须先更新当前移除注入规格和对应验收标准。
