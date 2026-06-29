# 验收标准：Codex 启动/重启按钮重新拉起修复

验证对象：`spec/codex-restart-command-relaunch.md`

## 验收项

1. 文档存在
   - 通过标准：本文件与 `spec/codex-restart-command-relaunch.md` 存在。
   - 证据：文件存在检查。

2. 顶部按钮仍调用 restart 动作
   - 通过标准：顶部“启动/重启Codex”按钮仍绑定 `actions.restartCodex()`。
   - 证据：`windows_subsystem` 回归测试。

3. restart command 使用专用 launcher 清理逻辑
   - 通过标准：`restart_claude_codex_pro` 中调用 `stop_launcher_processes_for_codex_restart()`。
   - 证据：`windows_subsystem` 回归测试。

4. restart command 保留关闭旧 Codex
   - 通过标准：`restart_claude_codex_pro` 中仍调用 `stop_codex_processes()`。
   - 证据：`windows_subsystem` 回归测试。

5. restart command 保留重新启动 launcher
   - 通过标准：`restart_claude_codex_pro` 中仍调用 `spawn_claude_codex_pro_launch(...)`。
   - 证据：`windows_subsystem` 回归测试。

6. 前端类型检查通过
   - 通过标准：`npm --prefix apps/claude-codex-pro-manager run check` 成功。
   - 证据：命令输出。

7. 前端生产构建通过
   - 通过标准：`npm --prefix apps/claude-codex-pro-manager run vite:build` 成功。
   - 证据：命令输出。

8. Manager UI 回归测试通过
   - 通过标准：`cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem -- --nocapture` 成功。
   - 证据：命令输出。

9. Debug 管理工具重新构建
   - 通过标准：`cargo build -p claude-codex-pro-manager --manifest-path Cargo.toml` 成功。
   - 证据：命令输出。
