# 验收标准：Claude 白屏诊断与兜底修复

验证对象：`spec/claude-white-screen-diagnostics.md`

## 验收项

1. 文档存在
   - 通过标准：`spec/claude-white-screen-diagnostics.md` 存在。
   - 通过标准：`acceptance/claude-white-screen-diagnostics.md` 存在。

2. 白屏风险入口被兜底
   - 通过标准：`open_claude_chinese_window` 不再直接使用 `WebviewUrl::External(url)` 打开 `https://claude.ai/new`。
   - 通过标准：后台命令包含本地 HTML 壳生成逻辑。
   - 通过标准：本地 HTML 壳包含加载状态、白屏说明、`https://claude.ai/new` 和“在浏览器打开 Claude”入口。

3. 不恢复已删除入口
   - 通过标准：管理工具前端仍不包含 `onClick={() => void actions.openClaudeChinese()}`。
   - 通过标准：管理工具前端仍不展示“包装 WebView”。

4. 不破坏既有链路
   - 通过标准：`claude-codex-pro://plugin-hub` 导航拦截仍存在。
   - 通过标准：Claude 中文注入脚本不被修改。

5. 真实验证
   - 通过标准：运行 manager 定向测试通过。
   - 通过标准：`npm --prefix apps/claude-codex-pro-manager run check` 通过。
   - 通过标准：`npm --prefix apps/claude-codex-pro-manager run vite:build` 通过。
   - 通过标准：`cargo build -p claude-codex-pro-manager --manifest-path Cargo.toml` 通过。

## 不在范围内

- 不验证 Claude 官方服务可用性。
- 不验证用户 Claude 账号登录状态。
- 不验证外部浏览器是否已登录 Claude。
