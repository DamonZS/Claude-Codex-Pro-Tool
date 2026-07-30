# 验收标准：Codex Matt Pocock Skills 第三方仓库

验证对象：`spec/codex-matt-pocock-skills-marketplace.md`

## 验收项

1. 状态列表包含 `Matt Pocock Skills 仓库`，名称为 `mattpocock-skills`。
2. 缺少 Matt 快照或配置时 `needsRepair=true`；快照和全部内置仓库配置完成后状态通过。
3. 修复写入 `[marketplaces.mattpocock-skills]`，使用本地 source，不写入 `[plugins.*]`。
4. 快照包含 Codex marketplace 清单、插件清单、`ask-matt` Skill，并使用 `ON_INSTALL` 认证策略。
5. 下载、转换和注册不执行上游脚本或 hooks，重复执行保持幂等。
6. Manager 预览数据和仓库回退列表包含 Matt 仓库。

## 必需验证

```powershell
cargo fmt --check
cargo test -p claude-codex-pro-core codex_plugin_marketplace -- --nocapture
cargo test -p claude-codex-pro-manager --test windows_subsystem plugin_memory_tools_ui_regression_is_locked_down -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo build -p claude-codex-pro-manager --release
```

## 失败条件

- 只显示 UI 卡片但没有本地快照和 config.toml 注册。
- 自动安装具体技能或执行第三方脚本。
- Matt 仓库缺失时整体状态仍显示正常。

## 非范围检查

- 不要求自动启用 `ask-matt` 或其他技能。
- 不要求修改 Claude 插件仓库。
