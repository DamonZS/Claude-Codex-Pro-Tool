# 验收标准：会话上下文正文显示回归修复

验证对象：`spec/session-context-viewer-regression.md`

## 通过/失败标准

1. 真实失败证据
   - 通过：指定 Codex rollout 能解析出 11 条 user/assistant 消息，且每条正文非空；问题被定位为共享展示层压缩，而非解析器返回空正文。
   - 失败：通过删除消息、伪造占位正文或缩减分页数量掩盖问题。

2. JSONL 解析夹具
   - 通过：由真实 rollout 结构脱敏提炼的夹具覆盖多段 `input_text`、`output_text`、应忽略的 developer 和非消息事件；完整上下文加载入口返回 5 条顺序正确且正文非空的消息。
   - 失败：测试绕过 `load_codex_session_context`，或夹具未覆盖真实事件的 `response_item/message` 嵌套结构。

3. 消息卡布局
   - 通过：`.claude-session-context-message` 明确禁用 Flex 纵向收缩，并允许在容器宽度内正常断行。
   - 失败：消息卡仍可被父级 Flex 压缩，或使用固定高度、最大高度、独立滚动条裁切正文。

4. Codex 与 Claude 一致性
   - 通过：两类会话继续复用同一消息列表和消息卡规则，点击任一会话均可查看正文。
   - 失败：只对某个角色或某一种会话添加临时特例。

5. 现有交互不回归
   - 通过：正文保留换行和长文本断行；弹窗整体滚动、加载更早内容、错误态和关闭交互的现有结构不变。
   - 失败：出现横向溢出、正文重叠、列表不可滚动或分页入口失效。

6. 自动化验证
   - 通过：Manager 会话页面 UI 契约测试、前端类型检查与生产构建通过。
   - 失败：相关命令退出码非 0，或契约测试没有覆盖消息卡禁用收缩。

## 必需验证与证据

```powershell
cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem session_context -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
```

证据包括指定 rollout 的消息数量/正文长度摘要、测试结果和构建退出码。验证输出不得包含完整私密会话正文。

## 非目标检查

- 不要求更改会话解析协议或数据库结构。
- 不要求修改主题、注入、Launcher 或应用更新流程。
- 不要求在列表首次加载时返回完整会话正文。
