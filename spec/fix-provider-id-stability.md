# 换供应商后历史会话可用（provider id 稳定化）

## 背景

用户在 CCP 里新建供应商并切换后，打开历史 Codex 会话直接报
`Model provider 'gpt' not found`。根因是 `config.toml` 里的 provider id 由 UI 侧派生：
新建供应商取 `uniqueSupplierProfileId(profiles, "provider")`（`provider` / `provider-2`），
预设供应商取预设名（GPT → `gpt`）。切换时 `complete_relay_profile_config` 只保留当前
provider 表，旧表被删；而历史会话的 `session_meta.payload.model_provider` 仍写着旧 id。
切换链路原来不调用 provider 同步，只有下一次正常启动 Codex（`launcher.rs` 的
`hooks.run_provider_sync()`）才会重写会话，所以旧会话在切换后到下一次启动之间全是孤儿。
用户现场即 `gpt` → `provider-2` 的切换，605 个会话停在 `gpt`。

## 目标

- provider id 只由中转地址派生：同一地址换 key、换配置不换 id，历史会话不会因换 key 断开。
- 确实换到另一个地址时，切换流程自动完成两件事：
  1. 把上一个 provider id 保留成指向新表的别名（同一 `base_url` / 鉴权），旧会话有落点；
  2. 把历史会话（rollout / sqlite / 全局状态）的 provider 改写成新 id。
- 上述两步失败不能阻断切换本身，只记事件日志。

## 非目标

- 不改供应商列表里用于内部区分的 profile id（仍是 `provider` / `provider-2` / 预设名），
  它只做 UI 主键，不再进入 `config.toml`。
- 不改 `complete_relay_profile_config` 的全量归一化语义。
- 不做「恢复会话」的真正 resume（按钮只读，文案已改为「查看会话」，另开 spec）。

## 技术约束

- 派生规则前后端必须一致：`claude_codex_pro_core::relay_config::provider_id_for_base_url`
  与 `apps/claude-codex-pro-manager/src/lib/supplier.ts` 的 `supplierIdForBaseUrl` 跑同一张表。
- 只取 host[:port]：忽略 scheme、路径、查询、userinfo；小写；非 `[a-z0-9]` 折叠成 `-`；
  结果前缀 `relay-`，形如 `relay-api-toporeduce-cn`、`relay-img-pqaai-cc`。
- 走机内代理时 `config.toml` 的 `base_url` 是 `127.0.0.1:<port>`，那不是中转身份，
  派生源必须取 `upstreamBaseUrl`。
- 写盘沿用 `write_codex_live_atomic`（校验 + 备份 + 失败回滚），沿用
  `preserve_computer_use_guard` 开关。
- core 不能依赖 data，会话迁移只能在 manager 层调用
  `run_provider_sync_with_target_waiting`，等待上限 10s。

## 数据与接口

- `provider_id_for_base_url(&str) -> Option<String>`
- `active_provider_id_from_home(&Path) -> Option<String>`
- `keep_provider_alias_in_home(&Path, alias, source, preserve_computer_use_guard) -> Result<bool>`
  （别名已存在或源表不存在时返回 `false`，不重复写）
- `migrate_sessions_after_provider_id_change(home, previous_provider_id, preserve_computer_use_guard)`
- 事件：`manager.switch_relay_profile.alias_failed`、`manager.switch_relay_profile.session_sync`。

## 交付范围

- `crates/claude-codex-pro-core/src/relay_config.rs`
- `apps/claude-codex-pro-manager/src/lib/supplier.ts`、`supplier.test.ts`
- `apps/claude-codex-pro-manager/src-tauri/src/commands.rs`
- `apps/claude-codex-pro-manager/src-tauri/tests/windows_subsystem.rs`
- `apps/claude-codex-pro-manager/src/screens.tsx`（恢复按钮文案）