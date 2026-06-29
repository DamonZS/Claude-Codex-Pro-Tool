# 验收标准：管理工具前端 UI 重设计

验证对象：`spec/manager-ui-redesign.md`

## 验收项

1. 规格与验收文档存在。
   - 通过标准：`spec/manager-ui-redesign.md` 与 `acceptance/manager-ui-redesign.md` 存在。
   - 证据：文件存在检查。

2. 全局深色控制台视觉系统已建立。
   - 通过标准：`styles.css` 包含统一 CSS 变量，覆盖背景、表面、边框、文字、主色、状态色、半径和控件样式。
   - 证据：源码检查。

3. 页面结构和业务功能未被重写。
   - 通过标准：现有路由、主要按钮、Tauri command 调用、工具与插件页、供应商页、设置页仍存在。
   - 证据：`windows_subsystem` 回归测试和源码检查。

4. 管理工具页面不再出现顶部后端链接胶囊。
   - 通过标准：`App.tsx` 不渲染“后端链接”顶部胶囊或 `ops-topbar-pill`。
   - 证据：现有 UI 回归测试。

5. 高密度控件在新版主题中保持对齐。
   - 通过标准：上下文条目 actions、toggle、编辑、删除按钮仍使用固定布局；工具与插件页开关轨道和滑块在固定操作槽内对齐；状态行和仓库行允许换行但不遮挡按钮。
   - 证据：CSS / 源码检查与前端构建。

6. 供应商编辑按钮使用单只倾斜笔。
   - 通过标准：供应商列表编辑按钮使用 `Pencil` 和 `tilted-pen-icon`，不使用 `PencilRuler`。
   - 证据：`windows_subsystem` 回归测试和源码检查。

7. 前端类型检查通过。
   - 通过标准：`npm --prefix apps/claude-codex-pro-manager run check` 成功。
   - 证据：命令输出。

8. 前端生产构建通过。
   - 通过标准：`npm --prefix apps/claude-codex-pro-manager run vite:build` 成功。
   - 证据：命令输出。

9. Manager UI 回归测试通过。
   - 通过标准：`cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem -- --nocapture` 成功。
   - 证据：命令输出。

10. Debug 管理工具已重新构建。
   - 通过标准：`cargo build -p claude-codex-pro-manager --manifest-path Cargo.toml` 成功。
   - 证据：命令输出。

## 必需验证

至少运行：

```powershell
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem -- --nocapture
cargo build -p claude-codex-pro-manager --manifest-path Cargo.toml
```

## 不在范围内

- 新增后端命令。
- 改变 Codex / Claude / 插件 / 记忆业务逻辑。
- 引入新依赖。
- 修改 Claude 中文注入脚本。
