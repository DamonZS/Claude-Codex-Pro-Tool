# 验收标准：Codex 供应商独立启动持久化

验证对象：`spec/codex-supplier-independent-launch-persistence.md`

## 验收项

1. `relay_config::default_codex_home_dir()` 与 Codex 会话读取路径都优先使用 `CODEX_HOME`。
2. 供应商切换写入的 `config.toml` 和 `auth.json` 位于同一 Codex home，且保留原子写入与备份行为。
3. 管理工具退出、没有 provider sync 或 CCP launcher 运行时，重新读取该 home 仍能解析活动 provider 和认证结构。
4. API Key 不出现在测试输出、诊断日志或验收报告中。

## 必需验证

- `cargo test -p claude-codex-pro-core --lib relay_config -- --nocapture`
- `cargo fmt --check`
- `git diff --check`
- `cargo build --release`

## 完成证据

- `CODEX_HOME` 优先级回归测试通过。
- 默认产物 `target/release/claude-codex-pro.exe` 存在且为本次构建生成。
- 独立启动限制（运行中的 Codex 需要重启加载）在交付说明中明确。
