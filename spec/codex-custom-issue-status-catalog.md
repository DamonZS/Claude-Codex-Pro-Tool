# Codex 内嵌工作区自定义任务状态目录

## 背景

上游 Multica 的 `packages/core/types/issue-status.ts` 将任务状态建模为可扩展目录：
七个固定状态类别决定行为，而自定义状态使用稳定 `key`、可编辑名称和颜色，并归属于其中一个类别。
本地看板目前只接受七个固定状态键，导致上游返回或用户保存的自定义状态无法被真实表示。

## 目标

- 在本地控制面持久化上游兼容的状态目录条目。
- 保留七个系统状态；自定义状态只能归属其中一个类别。
- Issue 可以保存目录中已启用的自定义 `status`，同时保留 `status_category` 和 `status_name` 投影。
- 看板仍以七个类别为列；自定义状态显示在其所属类别列中，拖放到一列时写入该类别的系统状态键。
- 目录缺失或状态未知时，不伪造状态名称或改变原始 `status`；界面明确显示原始键。

## 非目标

- 不创建第二个 Codex Runtime、daemon、CLI 或执行器。
- 不修改 Codex 原生线程、项目、模型或配置。
- 不实现上游云端的跨工作区权限、同步或附件功能。

## 技术约束

- 以 `D:\Project\multica-upstream\packages\core\types\issue-status.ts` 和
  `server/cmd/server/router.go` 的 `/api/issue-statuses` 路由为字段和行为依据。
- 所有目录写入走现有本地工作区 revision/CAS 与原子写入机制。
- 系统状态不可重命名、重分类或归档；自定义状态 key 与 category 不可变。
- 目录和 Issue 的输入均限制为可渲染的文本及受控颜色值。

## 交付范围

- `MulticaWorkspaceResourceKey` 的状态目录资源、验证、默认系统目录和测试。
- 注入界面的状态目录管理入口、Issue 编辑器状态选择和七列看板分类渲染。
- 对应验收与针对性 Rust/JavaScript 回归测试。
