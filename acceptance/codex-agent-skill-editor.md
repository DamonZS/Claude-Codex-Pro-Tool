# Codex Agent Skill 编辑器验收

对应规格：`spec/codex-agent-skill-editor.md`。

1. 编辑已有 Agent 时显示 Skills 搜索、多选和保存控件；搜索无匹配显示空态。
2. 已绑定 Skill 初始勾选；保存构造完整选择集并调用 `/multica/skills/bindings/replace`，传入 `scopeKind=agent`、Agent ID 与当前 revision。
3. 只读或不可派发的 Skill 不能被勾选；页面未声明可执行 Skill 协议时，保存被禁用并显示准确降级文案。
4. 新建 Agent 使用独立的 `/multica/agents/create` 创建路径，并在同一持久化事务中创建 Agent 与所选 Skill 绑定；已有 Agent 编辑仍使用 replace-all 路径。
5. `node --check assets/inject/renderer-inject.js`、前端检查和 bridge 静态回归测试通过。
