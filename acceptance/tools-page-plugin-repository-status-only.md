# 验收标准：工具与插件页插件仓库自动修复

验证对象：`spec/tools-page-plugin-repository-status-only.md`

## 验收项

1. 规格文档存在且为中文可读内容。
   - 通过标准：`spec/tools-page-plugin-repository-status-only.md` 存在，并描述本次自动修复范围。
   - 证据：文件存在检查或源码检查。

2. 验收标准存在且为中文可读内容。
   - 通过标准：`acceptance/tools-page-plugin-repository-status-only.md` 存在，并列出可验证标准。
   - 证据：文件存在检查或源码检查。

3. 工具与插件页显示 Codex 插件仓库状态卡。
   - 通过标准：页面包含 Codex 插件仓库状态、仓库列表、刷新按钮和 Codex 修复插件仓库按钮。
   - 证据：源码检查或截图。

4. 工具与插件页显示 Claude 插件仓库状态卡。
   - 通过标准：页面包含 Claude 插件仓库状态、仓库列表、刷新按钮和 Claude 修复插件仓库按钮。
   - 证据：源码检查或截图。

5. 页面不显示打开仓库按钮。
   - 通过标准：Codex 仓库卡不显示“打开 OpenAI 插件仓库”；Claude 仓库卡不显示“打开 Claude 官方插件仓库”。
   - 证据：源码检查或截图。

6. Codex 修复按钮具备实际修复效果。
   - 通过标准：Codex 修复按钮调用 `repair_codex_plugin_marketplace`，后端继续下载、校验并注册 OpenAI 本地插件仓库到 `config.toml`。
   - 证据：源码检查与 Rust 测试。

7. Claude 修复按钮具备实际修复效果。
   - 通过标准：Claude 修复按钮调用真实写入配置的命令；该命令写入开发模式配置并确保 Anthropic 官方仓库与 Ponytail 仓库出现在 Claude 已知插件仓库配置中。
   - 证据：源码检查与 Rust 测试。

8. Claude 一键开发模式会同步写入插件仓库。
   - 通过标准：`configure_claude_desktop_dev_mode` 流程调用插件仓库写入逻辑，状态检查能识别已配置仓库。
   - 证据：源码检查与 Rust 测试。

9. 工具与插件页不显示插件目录市场。
   - 通过标准：`ToolsAndPluginsScreen` 不渲染 `PluginHubScreen` 或等价插件市场浏览/安装详情 UI。
   - 证据：源码检查。

10. MCP、Skills、Plugins 配置管理仍保留。
    - 通过标准：Codex 和 Claude 的 `ContextManagerPanel` 仍在工具页渲染。
    - 证据：源码检查或截图。

11. toggle 和编辑按钮显示正常。
    - 通过标准：上下文条目行中 toggle、编辑按钮、删除按钮尺寸稳定、无重叠；编辑按钮使用倾斜笔图标而非 `PencilRuler`。
    - 证据：源码检查、截图或 CSS 检查。

12. 前端类型检查通过。
    - 通过标准：`npm --prefix apps/claude-codex-pro-manager run check` 成功。
    - 证据：命令输出。

13. 前端构建通过。
    - 通过标准：`npm --prefix apps/claude-codex-pro-manager run vite:build` 成功。
    - 证据：命令输出。

14. 相关 Rust 测试通过。
    - 通过标准：覆盖 Codex/Claude 插件仓库配置的 Rust 测试通过。
    - 证据：命令输出。

## 必需验证

至少运行：

```powershell
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml codex_plugin_marketplace -- --nocapture
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml claude_desktop_marketplace -- --nocapture
```

如改动 Tauri command 注册，还应运行：

```powershell
cargo build -p claude-codex-pro-manager --manifest-path Cargo.toml
```

## 不在范围内

- 自动安装具体 Claude 官方插件。
- 自动信任第三方 hooks。
- 删除插件中心后端能力。
- 修复与本任务无关的供应商、翻译或发布问题。
