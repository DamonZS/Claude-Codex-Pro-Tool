# Codex 供应商配置一致性验收

对应规格：`spec/codex-provider-config-consistency.md`

## 通过标准

1. `cargo test -p claude-codex-pro-core relay_config -- --nocapture` 通过。
2. 回归测试证明活动 provider 缺表、删除当前 provider、动态 provider ID 均不会产生孤立 `model_provider`。
3. `cargo fmt --check` 通过。

## 非目标

- 不修改 Codex 原生会话数据、auth.json 中的用户登录令牌。
- 不改变供应商 UI 结构或供应商鉴权协议。
