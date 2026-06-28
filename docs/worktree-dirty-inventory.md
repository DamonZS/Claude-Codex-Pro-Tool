# 工作区脏改动分组清单

## 背景

当前工作区包含多轮功能修复、测试补充、流程文档和本地运行残留。为避免误回滚用户已有工作，本清单只做分组与提交建议，不把无法确认归属的功能改动直接删除。

## 分组建议

### 1. Harness Engineering 流程文档

建议作为独立提交。

- `AGENTS.md`
- `docs/harness-engineering-theory.md`
- `spec/harness-engineering-skill.md`
- `spec/harness-engineering-theory.md`
- `acceptance/harness-engineering-skill.md`
- `acceptance/harness-engineering-theory.md`
- `skills/harness-engineering/**`

说明：这些文件建立项目级工作流、理论说明和可复用 skill，和运行时代码无直接耦合。

### 2. 概览页、盘古记忆与注入状态回归修复

建议作为独立提交。

- `apps/claude-codex-pro-manager/src/App.tsx`
- `apps/claude-codex-pro-manager/src/styles.css`
- `apps/claude-codex-pro-manager/src/tauriBridge.ts`
- `assets/inject/renderer-inject.js`
- `crates/claude-codex-pro-core/src/memory_assist.rs`
- `crates/claude-codex-pro-core/src/status.rs`
- `crates/claude-codex-pro-core/tests/cdp_bridge.rs`
- `spec/overview-memory-injection-repair-regression.md`
- `acceptance/overview-memory-injection-repair-regression.md`

说明：这一组对应概览状态、盘古记忆状态、修复按钮反馈和 Codex 注入锚点。

### 3. Claude 开发模式、第三方代理与端口治理

建议作为独立提交。

- `apps/claude-codex-pro-manager/src-tauri/src/commands.rs`
- `apps/claude-codex-pro-manager/src-tauri/src/lib.rs`
- `apps/claude-codex-pro-manager/src-tauri/tests/windows_subsystem.rs`
- `acceptance/review-runtime-correctness-fixes.md`
- `crates/claude-codex-pro-core/src/claude_desktop.rs`
- `crates/claude-codex-pro-core/src/claude_desktop_provider.rs`
- `crates/claude-codex-pro-core/src/launcher.rs`
- `crates/claude-codex-pro-core/src/plugin_hub.rs`
- `crates/claude-codex-pro-core/src/protocol_proxy.rs`
- `crates/claude-codex-pro-core/src/relay_config.rs`
- `crates/claude-codex-pro-core/src/settings.rs`
- `crates/claude-codex-pro-core/tests/claude_desktop_provider.rs`
- `crates/claude-codex-pro-core/tests/launcher.rs`
- `crates/claude-codex-pro-core/tests/protocol_proxy.rs`

说明：这一组对应 Claude 一键开发模式、实际代理端口写入、端口冲突避让、helper 验证和供应商配置持久化。

### 4. Claude 汉化补丁稳定性

建议作为独立提交。

- `crates/claude-codex-pro-core/src/claude_zh_patch.rs`
- `crates/claude-codex-pro-core/tests/installers.rs`

说明：这一组对应中文资源获取超时回退、旧补丁清理、JS 校验和缓存清理。

### 5. 工作区清理与脏数据防护

建议作为独立提交。

- `.gitignore`
- `apps/claude-codex-pro-launcher/src/main.rs`
- `spec/worktree-dirty-cleanup.md`
- `acceptance/worktree-dirty-cleanup.md`
- `docs/worktree-dirty-inventory.md`

说明：这一组只处理误入仓库的本地运行数据、忽略规则和脏改动分组说明。

## 保留不动的内容

- 已跟踪源码中的功能改动：当前可以通过基础验证，不能在未确认业务目标时回滚。
- 测试中的 `sk-test`、`sk-test-secret`、`sk-test-redacted` 等字符串：当前为测试夹具或示例占位，不是本机真实密钥。
- 用户本地 Codex / Claude / 供应商 / 盘古记忆配置：本轮未读取或修改。

## 已清理内容

- `Files/WindowsApps/Claude_1.15962.0.0_x64__pzs8sxrjxfjjc` 本地运行日志已移出 Git 状态。
- `.gitignore` 已添加 `/Files/`，防止同类本地运行残留再次进入未跟踪列表。
- `apps/claude-codex-pro-manager/src-tauri/src/commands.rs` 文件头 BOM 已清理。
