# 验收标准：应用更新后启动路径自愈

验证对象：`spec/stale-app-path-self-healing.md`

## 验收项

1. 文档存在
   - 通过标准：本 spec 与验收文档均存在。
   - 证据：git diff 或文件存在检查。

2. Codex 启动请求校验非空旧路径
   - 通过标准：`normalize_launch_request` 不再直接信任非空 `app_path`；只有路径通过真实可执行文件检查时才继续使用，否则调用当前路径发现。
   - 证据：`windows_subsystem` 定向测试通过。

3. Codex 路径发现能避开旧版本目录
   - 通过标准：`current_codex_app_path_for_launch` 优先运行中路径，并对最新状态、用户保存路径、候选发现结果做可执行文件存在检查。
   - 证据：`windows_subsystem` 定向测试通过。

4. Claude 启动/重启跳过失效 hint
   - 通过标准：`launch_claude_desktop_app` 只尝试存在的 `executable_hint`，失效时继续候选发现和系统入口回退。
   - 证据：`windows_subsystem` 定向测试与 `claude_desktop_candidate` 测试通过。

5. 修复前端连接真实重启 Codex
   - 通过标准：`repair_frontend_connection` 不再用旧 runtime 心跳直接判成功；必须调用 `restart_codex_for_frontend_repair`，等待 Codex CDP 与 helper 后端自启完成，再等待不早于本次修复开始时间的新 `renderer.script_loaded` 或 `renderer.memory_runtime` 心跳，或明确返回失败。
   - 证据：`windows_subsystem` 定向测试通过。

6. Codex 进程识别覆盖非 WindowsApps 安装
   - 通过标准：`stop_codex_processes` 使用的进程识别能识别可归一化为有效 Codex 安装目录的 `codex.exe`，不只匹配 `WindowsApps\OpenAI.Codex_` 路径。
   - 证据：`watcher` 定向测试通过。

7. 构建产物更新
   - 通过标准：debug manager 成功重新构建，且 `target/debug/claude-codex-pro-manager.exe` 时间戳更新。
   - 证据：构建命令输出与文件时间戳。

## 验证命令

```bash
cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem codex_restart -- --nocapture
cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem claude_restart -- --nocapture
cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem frontend_connection -- --nocapture
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml --test watcher -- --nocapture
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml claude_desktop_candidate -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo build -p claude-codex-pro-manager --manifest-path Cargo.toml
```
