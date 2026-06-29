# 验收标准：Codex 第三方插件仓库接入

验证对象：`spec/codex-third-party-plugin-repository.md`

## 验收项

1. 规格与验收文档存在。
   - 通过标准：`spec/codex-third-party-plugin-repository.md` 与 `acceptance/codex-third-party-plugin-repository.md` 存在。
   - 证据：文件存在检查。

2. Codex 第三方 marketplace 配置写入正确。
   - 通过标准：修复逻辑写入 `[marketplaces.awesome-codex-plugins]`，字段包含 `source_type = "git"`、正确 source、`ref = "main"` 和两个 sparse path。
   - 证据：`cargo test -p claude-codex-pro-core --manifest-path Cargo.toml codex_plugin_marketplace -- --nocapture`。

3. 状态检查能发现第三方仓库缺失。
   - 通过标准：只配置 OpenAI 官方仓库时，Codex 插件仓库状态仍标记为需要修复；配置第三方仓库后状态通过。
   - 证据：核心单元测试。

4. 工具与插件页显示第三方仓库状态。
   - 通过标准：Codex 插件仓库卡显示 `awesome-codex-plugins` 和 GitHub URL。
   - 证据：`windows_subsystem` 回归测试和源码检查。

5. 不自动安装具体第三方插件。
   - 通过标准：代码只写 marketplace 配置，不写 `[plugins.*]` 第三方插件启用项。
   - 证据：源码检查。

6. 前端与 Manager 回归验证通过。
   - 通过标准：前端类型检查、前端构建、Manager UI 测试和 debug manager 构建成功。
   - 证据：命令输出。

## 必需验证

至少运行：

```powershell
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml codex_plugin_marketplace -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem -- --nocapture
cargo build -p claude-codex-pro-manager --manifest-path Cargo.toml
```

## 不在范围内

- 自动安装第三方插件。
- 自动信任第三方 hooks。
- 修改 Claude 中文注入脚本。
