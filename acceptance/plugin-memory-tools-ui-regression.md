# 验收标准：插件仓库、盘古记忆与工具页 UI 回归修复

验证对象：`spec/plugin-memory-tools-ui-regression.md`

## 验收项

1. 规格与验收文档存在。
   - 通过标准：`spec/plugin-memory-tools-ui-regression.md` 与 `acceptance/plugin-memory-tools-ui-regression.md` 存在。
   - 证据：文件存在检查。

2. Codex 官方插件仓库展示不再被隐藏过滤器误删。
   - 通过标准：注入脚本包含官方 marketplace 别名/扩展逻辑，并有 `cdp_bridge` 测试覆盖 `openai-curated` / `openai-api-curated` 搜索响应扩展和分词搜索。
   - 证据：源码检查与 `cargo test -p claude-codex-pro-core --manifest-path Cargo.toml --test cdp_bridge -- --nocapture`。

3. Codex 官方插件仓库配置兼容新旧仓库名。
   - 通过标准：修复逻辑会同时写入 `[marketplaces.openai-curated]` 与 `[marketplaces.openai-api-curated]`；本地注入数据会暴露两个 marketplace 名称；状态检查能发现只注册其中一个的半修复状态。
   - 证据：`cargo test -p claude-codex-pro-core --manifest-path Cargo.toml codex_plugin_marketplace -- --nocapture`。

4. 盘古记忆不会把会话标题当作对话记录。
   - 通过标准：注入脚本的对话提取限定主内容区并过滤导航/侧边栏；没有真实用户消息时不调用 `/memory/session`；测试覆盖该选择器/过滤逻辑。
   - 证据：源码检查与 `cdp_bridge` 测试。

5. 管理工具所有页面去掉顶部后端链接。
   - 通过标准：`App.tsx` 不再渲染“后端链接”胶囊或 `backend-chip` 顶部元素；测试断言不存在。
   - 证据：源码检查与 `windows_subsystem` 测试。

6. 工具与插件页明确展示官方仓库状态。
   - 通过标准：Codex 仓库卡显示“Codex 官方仓库 / openai-curated”状态；Claude 仓库卡逐条显示 Claude 官方仓库和 Ponytail 仓库 configured 状态；长文本不会遮挡或单行截断关键信息。
   - 证据：源码/CSS 检查与前端构建。

7. MCP / Skills / Plugins 条目开关和操作按钮对齐。
   - 通过标准：条目行右侧 actions 使用固定布局，toggle、编辑、删除控件尺寸稳定且不重叠。
   - 证据：CSS 检查、源码检查或截图。

8. 前端检查与构建通过。
   - 通过标准：`npm --prefix apps/claude-codex-pro-manager run check` 与 `npm --prefix apps/claude-codex-pro-manager run vite:build` 成功。
   - 证据：命令输出。

9. Manager 和核心回归测试通过。
   - 通过标准：相关 `windows_subsystem` 与 `cdp_bridge` 测试通过，必要时 `cargo build -p claude-codex-pro-manager --manifest-path Cargo.toml` 成功。
   - 证据：命令输出。

## 必需验证

至少运行：

```powershell
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml --test cdp_bridge -- --nocapture
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml codex_plugin_marketplace -- --nocapture
cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem -- --nocapture
cargo build -p claude-codex-pro-manager --manifest-path Cargo.toml
```

## 不在范围内

- 自动安装具体插件。
- 清空或迁移用户盘古记忆数据库。
- 修改 Claude 中文注入脚本。
