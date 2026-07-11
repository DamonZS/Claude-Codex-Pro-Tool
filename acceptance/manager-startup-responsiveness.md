# 验收标准：管理工具启动与导航响应性修复

验证对象：`spec/manager-startup-responsiveness.md`

## 验收项

1. 启动命令不阻塞 UI 线程
   - 通过标准：`load_settings` 和 `load_claude_chinese_window_status` 均为 async Tauri command，耗时主体通过 `tauri::async_runtime::spawn_blocking` 执行。
   - 证据：Manager 回归测试与源码检查。

2. 失败结果完整
   - 通过标准：blocking 任务 join 失败时返回 `failed` 状态及结构完整的设置/Claude 汉化状态 payload。
   - 证据：源码检查和 Rust 测试。

3. 真实窗口响应
   - 通过标准：结束旧管理器、重建并启动后，连续采样窗口 `Responding=True`；点击导航后仍保持响应。
   - 证据：PowerShell 进程采样和管理器日志中的 action result。

4. 功能不回归
   - 通过标准：前端类型检查、生产构建、Manager Windows 回归测试和 debug 构建通过。
   - 证据：命令输出。

## 必需验证

```powershell
cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem manager_startup_commands_run_blocking_work_off_ui_thread -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo build -p claude-codex-pro-manager --manifest-path Cargo.toml
```

## 不在范围内

- Codex/Claude 启动或注入修复。
- 盘古记忆算法和数据迁移。
- 供应商、插件或发布流程重构。
