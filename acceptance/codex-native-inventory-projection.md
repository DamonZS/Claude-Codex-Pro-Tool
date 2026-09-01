# Codex 原生会话与智能体只读投影验收

验证对象：`spec/codex-native-inventory-projection.md`

## 通过标准

1. “我的任务”在 Issue 看板之外显示“Codex 原生会话”区域；存在原生会话行时显示真实标题和 thread ID，不存在时显示明确空态。
2. 会话条目来源仅为 `[data-app-action-sidebar-thread-id]`，点击调用原生行行为，不修改 URL、React store 或 Multica 数据。
3. 智能体区域只显示本地已定义且有执行绑定/能力证据的条目；无证据时不生成假智能体。
4. 原生会话 DOM 暂时不可读时，看板仍可用且不显示“无任务”替代错误。
5. 会话数量最多 100 条，标题和 ID 使用文本节点渲染，不执行 HTML 或脚本。
6. Issue/Project/Agent/Squad/Autopilot 编辑器可读写上游核心字段（分配、父子关系、项目负责人、运行模式、调度执行者等），旧 camelCase 数据编辑后不丢失；敏感字段仍被拒绝进入渲染层。

## 验证方式

- Rust 注入源码契约测试覆盖选择器、上限、只读渲染和失败降级。
- `npm --prefix apps/claude-codex-pro-manager run check`
- `cargo test -p claude-codex-pro-core --test cdp_bridge -- --nocapture`
- 手动在 Codex 页面创建/打开会话，进入“我的任务”，确认条目与左侧原生会话一致；点击后确认仍由 Codex 原生导航激活。
- 手动分别新建/编辑五类本地实体，检查保存响应中的 snake_case 字段与输入一致，并用旧别名记录编辑一次确认字段迁移成功。

## 非范围

未暴露 `thread/list`、`project/list` 或 `agent/list` Host 方法的 Codex 版本不要求后台枚举未渲染的历史会话或远程智能体。
