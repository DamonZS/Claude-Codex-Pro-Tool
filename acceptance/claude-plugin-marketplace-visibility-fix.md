# 验收标准：Claude 插件仓库可见性修复

验证对象：`spec/claude-plugin-marketplace-visibility-fix.md`

## 通过 / 失败标准

### A. 新版字段写入

通过：

- `ensure_claude_desktop_marketplaces_config` 写入 `allowedPluginMarketplaces` 数组。
- 数组包含：
  - `{ "source": "github", "repo": "anthropics/claude-plugins-official" }`
  - `{ "source": "github", "repo": "DietrichGebert/ponytail" }`
- 已有用户自定义仓库不会被删除。

失败：

- 仍只写入 `extraKnownMarketplaces`。
- 写入时清空用户已有仓库。

### B. 状态检测不误报

通过：

- 状态检测以 `allowedPluginMarketplaces` 为准。
- 只有旧 `extraKnownMarketplaces` 时不能判定为已配置。

失败：

- 旧字段仍会让管理工具显示“已写入”。

### C. 修复动作有实际效果

通过：

- `repair_claude_desktop_marketplaces` 写入前调用关闭 Claude Desktop 的逻辑，降低运行中配置回写覆盖风险。
- 修复后本机活跃 `Claude-3p/claude_desktop_config.json` 可读取到 `allowedPluginMarketplaces`。

失败：

- 修复动作只写非活跃路径。
- 修复后活跃配置仍无 `allowedPluginMarketplaces`。

### D. 安全边界

通过：

- 不删除 Claude 账号、缓存、偏好、插件数据或数据库。
- 不改 Codex 插件仓库逻辑。
- 不改 Claude 中文注入逻辑。

失败：

- 引入数据清理或无关重构。

### E. 验证

通过：

- `plugin_hub` 相关 Rust 测试通过。
- `windows_subsystem` 结构测试通过。
- 前端检查和构建通过。
- manager debug 构建通过，并报告 exe 时间戳。
- 本机配置检查能看到 `allowedPluginMarketplaces`。

## 必需验证命令

```powershell
cargo fmt --check
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml plugin_hub -- --nocapture
cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo build -p claude-codex-pro-manager --manifest-path Cargo.toml
git diff --check
```

如 debug manager exe 被占用，只允许终止 `claude-codex-pro-manager` 进程；不得终止 Codex。
