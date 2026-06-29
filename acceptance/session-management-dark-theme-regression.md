# 验收标准：会话管理列表暗色主题回归修复

验证对象：`spec/session-management-dark-theme-regression.md`

## 验收项

1. 文档存在
   - 通过标准：本文件与 `spec/session-management-dark-theme-regression.md` 存在。
   - 证据：文件存在检查。

2. 会话列表结构保留
   - 通过标准：页面仍包含 `codex-session-browser`、项目标题、会话行、右侧相对时间和删除按钮。
   - 证据：`windows_subsystem` 回归测试。

3. 浅色背景已移除
   - 通过标准：`styles.css` 不再包含 `background: #f3eeee;`。
   - 证据：`windows_subsystem` 回归测试。

4. 暗色主题恢复
   - 通过标准：`codex-session-browser` 使用暗色半透明背景或管理工具暗色变量。
   - 证据：CSS 检查和前端构建。

5. 前端类型检查通过
   - 通过标准：`npm --prefix apps/claude-codex-pro-manager run check` 成功。
   - 证据：命令输出。

6. 前端生产构建通过
   - 通过标准：`npm --prefix apps/claude-codex-pro-manager run vite:build` 成功。
   - 证据：命令输出。

7. Manager UI 回归测试通过
   - 通过标准：`cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem -- --nocapture` 成功。
   - 证据：命令输出。

8. Debug 管理工具重新构建
   - 通过标准：`cargo build -p claude-codex-pro-manager --manifest-path Cargo.toml` 成功。
   - 证据：命令输出。
