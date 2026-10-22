# 环回中转出站绕过环境代理

## 背景

- `reqwest` 在工作区启用了 `system-proxy`（`Cargo.toml:34`），`http_client` 的出站请求默认同时读取环境变量代理与 Windows 系统代理。
- 用户开启 Clash / verge-mihomo 之类代理后，**环回目标（本机中转）也会被截胡**：代理替「接受连接后不回包」的上游合成 `HTTP/1.1 502 Bad Gateway`，客户端拿到的 502 并非上游产生。
- 环回无法靠旁路列表自救：hyper-util 的 Windows 系统代理实现只认域名型 `ProxyOverride`，`127.*` 这类 IP 字面量永远不生效。
- 直接后果有两层：①本机中转的可用性受“是否开机代理”影响；②集成测试 `request_telemetry` 不 hermetic —— `http_client.rs` 的 `#[cfg(test)] no_proxy()` 在集成测试链接库时不生效（集成测试不会以 `cfg(test)` 编译库本身）。

## 目标

- 包含：relay 出站目标为环回时绕过环境/系统代理；供应商模型发现路径（`model_catalog.rs` 直连供应商 `base_url`）同样按环回判定直连；判定函数单一来源（`http_client`）；清理 `protocol_proxy.rs` 既有 `[DEBUG]` 输出；远端目标维持既有行为；`request_telemetry` 在“无代理”和“有代理”两种环境下确定性通过。
- 不包含：私网段（`10.`、`192.168.` 等）绕过；CDP 与更新下载的既有 `no_proxy` 约定；`settings.rs:4714` 与 `tests/uia_real_operation.rs:31` 的既有 warning；前端死 CSS 清理（供应商样式，另列）。

## 用户视角

开启代理时，本地中转的 relay 请求不再被代理截胡，不再拿到假 502；关闭代理时行为与改动前完全一致；远端供应商请求仍按用户代理设置走代理。

## 功能要求

1. **判定单一来源**：`http_client::url_target_is_loopback`（`pub(crate)`）解析 URL 后，host 为 `localhost`（忽略大小写）、`127.0.0.0/8` 或 `::1` 视为环回；解析失败或其它 host 一律视为非环回。所有出站调用点共用该函数，不得各自复制判定。
2. **生效位置**：relay 出站四处 —— responses 转发、models 列表、Claude Desktop messages 转发、Chat Completions 转发；再加供应商模型发现一处（`discover_relay_profile_model_ids`）。环回走 `no_proxy` 客户端，非环回维持既有客户端。
3. **远端行为不变**：`proxied_client` / `streaming_proxy_client` 的签名与语义保留（等价 `local = false` 薄包装），既有非 relay 调用点零改动。
4. **事件记录语义不变**：`request_telemetry` 的 `http_status` 仍只记录真实收到的上游状态码；环回直连时上游无响应即 `None`（本规格不改该语义，仅消除环境噪声）。

## 技术约束

- 不新增依赖，不改 `reqwest` 的 `system-proxy` 特性；UA 归一化与超时策略保持在 `http_client` 的单一构建点，环回策略不得散落到调用点。
- 判定与策略必须带注释说明原因（Windows `ProxyOverride` 的 IP 字面量失效 + 代理会合成 502）。

## 交付范围

- `crates/claude-codex-pro-core/src/http_client.rs`（判定函数与客户端工厂）
- `crates/claude-codex-pro-core/src/protocol_proxy.rs`（四处调用点 + 移除 `[DEBUG]` 输出）
- `crates/claude-codex-pro-core/src/model_catalog.rs`（模型发现路径）
- 证据：`request_telemetry` 在两种代理环境下均通过；`model_catalog` 在两种代理环境下均通过；`launcher` / `relay_config` / `--lib` 不回归；远端仍走代理的运行时对拍；判定函数边界单测。

## 变更记录

- 2026-10-22：初版（由 `request_telemetry` 既有失败的定性结论引出）。
- 2026-10-22：按用户裁决扩展 —— 判定函数上移到 `http_client`（`url_target_is_loopback`，测试随之上移）；纳入模型发现路径（`model_catalog.rs` 同类缺陷）；清理 `protocol_proxy.rs` 既有 `[DEBUG]` 输出。
