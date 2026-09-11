# 验收：Codex 路由就绪与删除确认

对应 spec/codex-route-readiness-and-delete-confirm.md。

1. Codex 路由写盘之前启动固定配置端口的 helper；启动失败返回错误。
2. 两个供应商切换入口均覆盖；关闭路由、官方登录、Claude 流程不新增代理依赖。
3. 隔离页面验证删除手势去重、取消、确认提交一次、成功移除行、失败保留行。
4. 桥接失联有超时结果，超时不自动再次提交删除。
5. 定向 Rust 测试、JS 行为测试、语法检查及 Release 构建通过。
6. 默认 target/release/claude-codex-pro.exe 更新时间晚于修改；现有进程保持运行。

真实 Codex UI、真实国模上游请求需另行记录实际验证情况；测试不删除用户会话、不切换用户当前供应商。

## 执行记录（2026-09-11）

- 现场日志：管理端 PID 16748 启动时记录 `helper.detached_port_conflict` / `manager.helper.detached_failed`，端口为 57321；后续国模切换仍记录成功。切换回 GPT 前的备份配置指向 `http://127.0.0.1:57321/v1`。只读监听检查显示仅启动器 PID 8672 的 55180 在线。临时绑定探测确认 57321 当前可以绑定，探测后立即释放。
- 删除复现：`scripts/test-delete-confirm.cjs` 在修改前失败，单手势产生两个确认框（`2 !== 1`）。修改后通过手势去重、取消、失败保留、桥接超时无自动重试、单次成功删除及行移除检查。
- 浏览器验证使用工作区运行时的 Playwright 和本机 Edge（headless），隔离页面中仅运行源码提取出的删除函数，桥接为内存模拟；未操作真实会话。
- `cargo test -p claude-codex-pro-manager --test windows_subsystem codex_supplier_switch_starts_route_proxy_before_writing_config -- --nocapture`：1 passed。
- `cargo test -p claude-codex-pro-core --lib responses_proxy_ -- --nocapture`：2 passed，本地模拟上游验证 Responses 透明转发与 Chat Completions 转换后仅发送一次请求。
- `cargo test -p claude-codex-pro-core --test launcher detached_helper_ -- --nocapture`：2 passed，覆盖 helper 启动与未验证端口占用错误。
- `cargo test -p claude-codex-pro-data --test storage_adapter delete_ -- --nocapture`：7 passed，临时数据库验证删除、撤销、多库删除与失败回滚。
- `cargo test -p claude-codex-pro-core --test cdp_bridge injection_script_restores_injected_session_delete -- --nocapture`：1 passed。
- `node --check assets/inject/renderer-inject.js`、`npm --prefix apps/claude-codex-pro-manager run vite:build`、`git diff --check`：通过。构建仅报告现有未使用代码及前端包体积提示。
- 验收 1-4 已通过代码检查与定向测试；真实客户端删除及国模上游请求尚未验证，旧进程正在继续运行。
- `cargo build --release`：成功，用时 1m 59s。验收 5-6 的构建与产物检查通过。
- 产物：`D:/Project/Claude-Codex-Pro-Tool/target/release/claude-codex-pro.exe`，2026-09-11 14:44:51，46,367,232 字节；SHA256 `E6E973A88177F99F9C34A9C373D78C11DC742E14AFACDDD83897E5F54338983E`。
- 二进制包含 `button.__codexDeletePending = true`、路由失败提示和 `model_catalog_json`。构建后 PID 16748 / 8672（旧 `claude-codex-pro2.exe`）及 PID 5028（Codex）仍存活。
- 追加根因修复：启动器 `DefaultLaunchHooks::start_helper` 先复用已响应的持久 helper，避免管理端已占用 57321 时重复绑定导致 Codex 启动失败。

浏览器验证命令（PowerShell）：

```powershell
$env:NODE_PATH = 'C:/Users/Damon/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules'
node scripts/test-delete-confirm.cjs
```
