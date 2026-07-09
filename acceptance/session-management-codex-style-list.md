# 验收标准：会话管理 Codex 风格会话列表

验证对象：`spec/session-management-codex-style-list.md`

## 验收项

1. 规格与验收文档存在。
   - 通过标准：`spec/session-management-codex-style-list.md` 和 `acceptance/session-management-codex-style-list.md` 存在。
   - 证据：文件存在检查。

2. Codex 和 Claude 会话列表按项目分组。
   - 通过标准：`SessionManagementScreen` 中存在项目分组逻辑，使用 `cwd` 等字段推导项目名，并渲染 Codex/Claude 两个会话管理面板的项目标题与项目下的会话。
   - 证据：源码检查和 UI 回归测试。

3. 会话展示接近 Codex 侧边栏结构。
   - 通过标准：页面包含 `codex-session-browser`、项目组、项目标题行、会话行、右侧相对时间等结构，并包含 `Claude 会话管理`。
   - 证据：`windows_subsystem` 回归测试和 CSS 检查。

4. 删除行为保持原功能。
   - 通过标准：会话行仍调用 `actions.deleteLocalSession(session)`，不修改后端删除接口。
   - 证据：源码检查。

5. 不新增后端命令或依赖。
   - 通过标准：不新增 Tauri command，不新增 npm 依赖。
   - 证据：源码和配置检查。

6. 前端类型检查通过。
   - 通过标准：`npm --prefix apps/claude-codex-pro-manager run check` 成功。
   - 证据：命令输出。

7. 前端生产构建通过。
   - 通过标准：`npm --prefix apps/claude-codex-pro-manager run vite:build` 成功。
   - 证据：命令输出。

8. Manager UI 回归测试通过。
   - 通过标准：`cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem -- --nocapture` 成功。
   - 证据：命令输出。

9. Debug 管理工具已重新构建。
   - 通过标准：`cargo build -p claude-codex-pro-manager --manifest-path Cargo.toml` 成功。
   - 证据：命令输出。

10. 会话管理页移除 Claude 会话诊断并使用宽屏布局。
    - 通过标准：源码中 `SessionManagementScreen` 不再包含 `Claude 会话诊断`、`launchClaudeDesktop`、`installClaudeZhPatch`；包含 `session-management-wide-grid`、`session-history-card`、`session-codex-card`、`session-claude-card`、`Codex 会话管理` 与 `Claude 会话管理`，且 Codex 列表在卡片内限高滚动。
    - 证据：Manager UI 回归测试和源码检查。

## 必需验证

至少运行：

```powershell
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem -- --nocapture
cargo build -p claude-codex-pro-manager --manifest-path Cargo.toml
```

## 不在范围内

- 修改 Codex 本地数据库结构。
- 修改删除会话后端行为。
- 新增会话打开、编辑、归档或批量操作。
- 修改 Claude 中文注入脚本。
