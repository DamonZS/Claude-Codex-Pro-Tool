# 供应商、客户端与会话页面背景材质验收

对应规格：`spec/overview-background-material.md`

## 通过标准

1. 供应商页的环境提示、开关行和供应商卡片不再使用白色/实体黑色背景，并与 CCP 深色透明液态玻璃边框、高光和阴影一致。
2. 客户端与增强页的左右主容器、详情分区和状态栏使用透明液态玻璃材质，内容结构和操作按钮保持不变。
3. 会话页的摘要、搜索/范围控件、Agent 筛选和会话列表使用透明液态玻璃材质，列表文字可读。
4. 输入框、代码/日志和其他长文本数据表面保持清晰，不被背景模糊影响。
5. 运行 `npm --prefix apps/claude-codex-pro-manager run check`、`npm --prefix apps/claude-codex-pro-manager run vite:build`、`cargo build --release -p claude-codex-pro-manager -j 2` 和 `git diff --check` 均通过。

## 非目标

- 不验证业务数据采集、会话解析、供应商路由逻辑或外层窗口布局。
- 不以浏览器预览代替真实构建验收。
