# 验收标准：Codex Product Design Skill 插件仓库自动注册

验证对象：`spec/codex-product-design-skill-marketplace-autoregister.md`

## 验收项

1. 规格与验收文档存在。
   - 通过标准：本文件与对应 spec 均存在。
   - 证据：文件存在检查。

2. Codex 修复逻辑写入 Product Design Skill 仓库。
   - 通过标准：修复逻辑写入 `[marketplaces.codex-skills-alternative]`，字段包含 `source_type = "local"`，source 指向管理工具生成的本地 marketplace 快照。
   - 证据：`cargo test -p claude-codex-pro-core --manifest-path Cargo.toml codex_plugin_marketplace -- --nocapture`。

3. 状态检查能发现 Product Design Skill 仓库缺失。
   - 通过标准：只配置 OpenAI 官方仓库和既有第三方仓库时，Codex 插件仓库状态仍标记为需要修复；配置 Product Design Skill 仓库后状态通过。
   - 证据：核心单元测试。

4. Product Design Skill 本地快照结构可被 Codex 识别。
   - 通过标准：本地快照包含 `.agents/plugins/marketplace.json`、`plugins/codex-skills-alternative/.codex-plugin/plugin.json` 和 `plugins/codex-skills-alternative/skills/product-design/SKILL.md`，marketplace 认证策略为 `ON_INSTALL`。
   - 证据：核心单元测试。

5. 管理工具启动后自动注册 Codex 插件仓库。
   - 通过标准：`App.tsx` 存在只执行一次的启动自动修复流程，调用 `refreshCodexPluginMarketplace(true)` 并在需要修复时调用 `repairCodexPluginMarketplace(true)`，不弹确认框。
   - 证据：`windows_subsystem` 回归测试。

6. 工具与插件页显示 Product Design Skill 仓库状态。
   - 通过标准：Codex 仓库卡显示 `Product Design Skill 仓库`、`codex-skills-alternative` 和 GitHub URL。
   - 证据：`windows_subsystem` 回归测试和源代码检查。

7. 不自动安装具体第三方插件。
   - 通过标准：修复逻辑只写 marketplace 配置，不写入 `[plugins."codex-skills-alternative@..."]` 或其他第三方插件启用项。
   - 证据：核心单元测试和源代码检查。

8. 基础验证通过。
   - 通过标准：核心 marketplace 测试、前端类型检查、前端构建、Manager UI 回归测试和 debug manager 构建成功。
   - 证据：命令输出。

## 必需验证

```powershell
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml codex_plugin_marketplace -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem -- --nocapture
cargo build -p claude-codex-pro-manager --manifest-path Cargo.toml
```

## 不在范围内

- 自动安装 `codex-skills-alternative@...`。
- 自动信任第三方 hooks。
- 修改 Claude 中文注入或 Codex 注入脚本。
