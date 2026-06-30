# 验收标准：管理工具未来空间概念 UI 预览

验证对象：`spec/manager-future-space-ui-preview.md`

## 验收项

1. 文档存在
   - 通过标准：`design/context.md`、`spec/manager-future-space-ui-preview.md`、`acceptance/manager-future-space-ui-preview.md` 存在。
   - 证据：文件存在检查。

2. 预览文件存在
   - 通过标准：`docs/ui-previews/future-space-manager/index.html` 存在。
   - 证据：文件存在检查。

3. 覆盖所有现有页面
   - 通过标准：预览包含概览、供应商、工具与插件、会话管理、维护、设置、关于 7 个页面入口和页面内容。
   - 证据：源码检查与浏览器截图。

4. 不应用到真实前端
   - 通过标准：本任务不修改真实应用的页面组件逻辑，不把概念稿写入 `apps/claude-codex-pro-manager/src/App.tsx`。
   - 证据：`git diff --stat` 与源码检查。

5. 不增删真实功能
   - 通过标准：预览只是静态模拟，不调用 Tauri command，不读写配置，不启动或终止进程。
   - 证据：源码检查。

6. 视觉方向符合要求
   - 通过标准：预览包含空间场景、半透明磨砂玻璃悬浮面板、3D 景深、柔和渐变光影、局部高饱和撞色和高级赛博轻奢质感。
   - 证据：截图与人工检查。

7. 内容布局不过度拥挤
   - 通过标准：桌面视口下主要面板文字、按钮、开关、状态徽标不重叠；页面可滚动。
   - 证据：浏览器截图。

8. 基础交互可用
   - 通过标准：点击导航能切换 7 个页面；预览开关可以切换视觉状态。
   - 证据：浏览器自动化或手动验证记录。

## 必需验证

至少执行：

```powershell
Test-Path docs/ui-previews/future-space-manager/index.html
```

并使用浏览器或等效工具打开预览，检查页面渲染和导航交互。

## 不在范围内

- 真实前端落地。
- 后端命令调用验证。
- 与真实 Codex / Claude 进程交互。
- 发布安装包。
