# Codex 原生会话与智能体只读投影验收

验证对象：`spec/codex-native-inventory-projection.md`

## 通过标准

1. “我的任务”在 Issue 看板之外显示“Codex 原生会话”区域；存在原生会话行时显示真实标题和 thread ID，不存在时显示明确空态。
2. 会话条目来源仅为 `[data-app-action-sidebar-thread-id]`，点击调用原生行行为，不修改 URL、React store 或 Multica 数据。
3. 智能体区域显示本地已定义且有执行绑定/能力证据的条目，并在有真实线程父子
   关系时显示只读原生子智能体；无证据时不生成假智能体。
4. Skills 区域显示来自 `~/.codex/skills` 的真实只读元数据；不渲染 Skill 正文、
   凭据或不可验证的执行能力。
5. 原生会话 DOM 暂时不可读时，看板仍可用且不显示“无任务”替代错误。
6. 会话、原生智能体和 Skills 数量最多 100 条，标题、ID 和描述使用文本节点渲染，
   不执行 HTML 或脚本。
7. Issue/Project/Agent/Squad/Autopilot 编辑器可读写上游核心字段（分配、父子关系、项目负责人、运行模式、调度执行者等），旧 camelCase 数据编辑后不丢失；敏感字段仍被拒绝进入渲染层。
8. 原生线程的项目归属使用 `project_roots.path` 与 `cwd` 的规范化最长路径匹配；大小写、分隔符和末尾分隔符差异不影响匹配，未匹配线程不被丢弃且不显示伪造项目。

## 验证方式

- Rust 注入源码契约测试覆盖选择器、上限、只读渲染和失败降级。
- `npm --prefix apps/claude-codex-pro-manager run check`
- `cargo test -p claude-codex-pro-core --test cdp_bridge -- --nocapture`
- 手动在 Codex 页面创建/打开会话，进入“我的任务”，确认条目与左侧原生会话一致；点击后确认仍由 Codex 原生导航激活。
- 手动分别新建/编辑五类本地实体，检查保存响应中的 snake_case 字段与输入一致，并用旧别名记录编辑一次确认字段迁移成功。
- Rust 单元测试使用临时路径样本覆盖精确匹配、子目录最长匹配、Windows 分隔符/大小写归一化和无匹配保留。

## 非范围

未暴露 `thread/list`、`project/list` 或 `agent/list` Host 方法的 Codex 版本不要求后台枚举未渲染的历史会话或远程智能体。
