# 换供应商后历史会话可用（provider id 稳定化）验收

对应规格：`spec/fix-provider-id-stability.md`

## 通过标准

1. `cargo test -p claude-codex-pro-core --lib relay_config::tests` 通过，含：
   - `provider_id_only_depends_on_the_relay_address`：同一张地址表（含 `127.0.0.1`、
     userinfo、大小写、空串）前后端一致；
   - `the_previous_provider_id_stays_usable_as_an_alias_after_a_switch`：切换后旧 id 成为
     指向新表的别名，根键不变，重复调用不二次写入。
2. `npm --prefix apps/claude-codex-pro-manager run check`（tsc）通过。
3. `npm test`（cwd `apps/claude-codex-pro-manager`）通过，含
   `derives the same provider id from the relay address as the core side`。
4. `cargo test -p claude-codex-pro-manager --test windows_subsystem` 通过，含：
   - `supplier_editor_generates_config_from_editable_supplier_id`：`buildSupplierConfigToml`
     由地址派生 id，不再回落到 profile 名/slug；
   - `switching_a_supplier_keeps_the_previous_provider_id_usable_for_old_sessions`：
     先补别名、再重写会话，且带 `changedSessionFiles` 事件；
   - 切换命令内 `migrate_sessions_after_provider_id_change` 先于
     `sync_codex_credential_environment`。
5. `cargo fmt --check` 通过。

## 验证证据

- 命令与结果记入 `dev/changelog.md`。
- 手工复现路径（用户现场）：`config.toml` 活动 provider 由 `gpt` 切到 `provider-2` 后，
  旧会话的 `model_provider = "gpt"` 仍可解析；重新打开旧会话不再报
  `Model provider 'gpt' not found`。

## 非目标

- 不覆盖「恢复会话」按钮真的拉起 Codex（另开 spec），本次只把文案改成「查看会话」。
- 不覆盖网关侧 `cc12h` 计费表达式错误（对方网关，不属本仓）。
- 不覆盖历史会话 provider 改写后的界面刷新与 AITracker 计数变化。