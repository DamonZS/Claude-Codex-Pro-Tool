# Claude Desktop 供应商切换修复验收

对应规格：`spec/fix-claude-desktop-provider-switch.md`

## 通过标准

- `cargo test -p claude-codex-pro-core settings -- --nocapture` 通过。
- `atomic_write_replaces_an_existing_file_repeatedly` 通过。
- `cargo fmt --check` 通过。
- 修改不改变供应商配置字段、备份和回滚行为。

## 证据

记录 Rust 测试、格式检查的退出状态，并检查工作区差异仅涉及原子写入替换与规格验收文档。

## 非目标

- 不删除用户的 Claude Desktop 配置或备份。
- 不改变供应商 API Key、模型列表或代理路由规则。
