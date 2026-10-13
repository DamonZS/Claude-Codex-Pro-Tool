# 验收标准：CCP 注入配置的 Codex 兼容性

验证对象：`spec/codex-injection-config-compatibility.md`

## 通过标准

1. `sanitize_common_config_contents` 输出不含 MCP server 的 `type`，也不含 `disable_response_storage`、`features.code_mode_tool_description_first` 和 `[features.guardianv2].thread_context`。
2. `normalize_config_text` 对上述字段执行同样清理，且输出仍为可解析 TOML。
3. 清理后保留 MCP 的 `command`、`args`、`enabled`、其他 feature 字段和 Guardian 的其他配置。
4. 覆盖根配置与多个 Profile，含 dotted key / inline table；普通废弃表字段的异常 TOML fallback 仍移除目标键。
5. 供应商预览及后端导入生成器不再生成 `disable_response_storage`。
6. 供应商配置继续把已提供的 API Key 持久化至 `experimental_bearer_token`，不要求 CCP 在 Codex 启动时运行。
7. 不使用真实用户配置作为测试输入或改写目标。

## 必需验证

- `cargo fmt --check`
- `cargo test -p claude-codex-pro-core --test relay_config -- --nocapture`
- `cargo test -p claude-codex-pro-core --test relay_switch -- --nocapture`
- `node scripts/test-supplier-config.cjs`
- `git diff --check`
- `cargo build --release`

## 完成证据

- 上述定向测试通过，并包含截图三个废弃字段以及 Profile、相邻字段保留的回归断言。
- 默认产物 `target/release/claude-codex-pro.exe` 存在。
- Codex 已运行时需重启以加载新写入的配置。

## 非目标

- 不验证 Codex 上游版本自身的所有配置选项。
- 不修改真实用户配置或清理其他 Codex 警告。
