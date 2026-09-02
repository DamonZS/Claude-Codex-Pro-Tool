# Skills 模块原生 Codex 清单投影

## 背景

上游 Multica 将 Skills 作为独立工作区模块。当前 CCP 已从本机 Codex 状态库读取原生 Skill 元数据，但只在“我的任务”附加区域展示；Skills 路由在 Host inventory 不可用时可能显示空集合，造成原生 Skill 不可见。

## 目标

- `skills` 路由在本地集合为空或不可用时展示当前 Codex 本机只读 Skill 清单。
- 原生清单保留来源、名称和摘要，并明确它不是可编辑的 Multica Skill 记录。
- 没有实时 Host 执行能力时，选择、绑定、审查和派发动作必须保持禁用。

## 约束

- 只读取 `codex_native_skills` 投影，不复制 Skill 正文、凭据或运行时路径。
- 不自动安装、执行、信任或修改原生 Skill。
- 本地可编辑 Skill 集合非空时优先展示本地集合，避免混合来源导致误写。

## 交付

- 更新注入层 Skills 路由的空态/降级数据源。
- 增加静态回归检查，确保原生清单进入 Skills 路由且动作保持只读。
