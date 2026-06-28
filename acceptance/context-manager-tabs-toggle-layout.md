# 验收标准：工具与插件上下文管理切换与开关布局修复

验证对象：`spec/context-manager-tabs-toggle-layout.md`

## 验收项

1. 规格文档存在。
   - 通过标准：`spec/context-manager-tabs-toggle-layout.md` 存在。
   - 证据：文件存在检查。

2. 验收标准存在。
   - 通过标准：`acceptance/context-manager-tabs-toggle-layout.md` 存在。
   - 证据：文件存在检查。

3. 开关布局不再被图标按钮样式覆盖。
   - 通过标准：上下文条目行只对编辑/删除图标按钮应用 32x32 图标按钮样式；`ToggleSwitch` 保持自身宽高和滑块样式。
   - 证据：源码/CSS 检查和截图或本地运行检查。

4. 标签切换减少重复计算。
   - 通过标准：`ContextManagerPanel` 对合并后的条目、分类列表和计数使用缓存或等价方式，点击标签不重复执行相同过滤逻辑。
   - 证据：源码检查。

5. 原有行为不丢失。
   - 通过标准：`MCP`、`Skills`、`插件` 标签、启用开关、编辑按钮、删除按钮仍存在。
   - 证据：源码检查或 UI 验证。

6. 项目静态检查通过。
   - 通过标准：前端类型检查通过。
   - 证据：`npm --prefix apps/claude-codex-pro-manager run check` 输出。

7. 构建产物通过。
   - 通过标准：前端构建通过。
   - 证据：`npm --prefix apps/claude-codex-pro-manager run vite:build` 输出。

## 必需验证

至少运行：

```powershell
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
```

如无法进行桌面截图验证，必须说明原因，并提供源码/CSS 与构建验证证据。

## 不在范围内

- 修改供应商页业务逻辑。
- 修改 MCP/Skill/Plugin 数据格式。
- 修改插件安装策略。
