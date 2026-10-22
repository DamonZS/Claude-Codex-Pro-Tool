# 验收：环回中转出站绕过环境代理

验证对象：`spec/relay-loopback-ambient-proxy-bypass.md`

## 通过 / 失败标准

1. **两种代理环境下 `request_telemetry` 全绿**
   - 命令 `cargo test -p claude-codex-pro-core --manifest-path Cargo.toml --test request_telemetry`
   - 通过：①无代理环境 `15 passed; 0 failed`；②`HTTP_PROXY`/`HTTPS_PROXY`/`ALL_PROXY=http://127.0.0.1:7897` 同命令同样 `15 passed; 0 failed`。
   - 参照（修前）：环境②为 `14 passed; 1 failed`，`panicked at request_telemetry.rs:601 ... left: Some(502), right: None`。
2. **相邻目标不回归**：`--test launcher`、`--test relay_config`、`--test model_catalog`、`--lib` 全部 0 failed（记录通过数）。
3. **远端仍走代理**：运行时对拍（本地假代理 + 不回包上游）：`local = false` 时收到代理合成的 `502`（证明代理被使用），`local = true` 时为传输错误（证明已绕过）。
4. **判定函数边界固定**：判定函数为 `crates/claude-codex-pro-core/src/http_client.rs` 的 `pub(crate) fn url_target_is_loopback`（`protocol_proxy.rs` 与 `model_catalog.rs` 共用，不各自复制）。环回 host（`localhost`、`127.0.0.1`、`127.9.9.9`、`[::1]`）为 true；非环回（`10.`、`192.168.`、公网域名、空串、非法 URL）为 false，由 `http_client::tests::url_target_is_loopback_covers_loopback_hosts_and_rejects_others` 覆盖。
5. **改动面受控**：`cargo fmt --all -- --check` = 0；仅 3 个宿主文件变化（`http_client.rs`、`protocol_proxy.rs`、`model_catalog.rs`）；非 relay 调用点 0 改动；无调试残留（`[DEBUG]` `eprintln` 残留 0 处、无探针文件）。
6. **模型发现路径同样按环回直连**：`crates/claude-codex-pro-core/src/model_catalog.rs:1044-1052` 的 `discover_relay_profile_model_ids` 使用 `crate::http_client::proxied_client_for(&profile.user_agent, crate::http_client::url_target_is_loopback(&source.base_url))`；`--test model_catalog` 在无代理与带 `127.0.0.1:7897` 代理两种环境下都 `20 passed; 0 failed`。
7. **无 `[DEBUG]` 残留**：`crates/claude-codex-pro-core/src/protocol_proxy.rs` 中 `grep -c 'DEBUG'` = 0（改动前 5 处 `eprintln!`：normalize 入口、call_id 收集、已知 call_ids 汇总、添加 call_id、无法匹配）。

## 必需验证方式

- 上述命令实跑，输出原文摘要贴回；`git diff --stat` 与 `git status --porcelain` 审查改动面；必要时对关键宿主源码（`http_client.rs`、`protocol_proxy.rs`、`model_catalog.rs`）做一次只读复核。
- 代理环境复跑：`HTTP_PROXY=HTTPS_PROXY=ALL_PROXY=http://127.0.0.1:7897` 下跑 `--test request_telemetry` 与 `--test model_catalog`；先证明代理存活（经该代理请求返回可观测的 HTTP 状态码）。
- 残留检查：`grep -c 'DEBUG' crates/claude-codex-pro-core/src/protocol_proxy.rs` 必须为 `0`。

## 证据要求

- 命令与原始输出摘要（含通过/失败计数、退出码）。
- diff 统计与文件清单。
- 对拍探针的说明（临时文件已删除，最终工作区不含）。
- 未验证项必须显式列为“未验证”，不得推断。

## 执行记录（2026-10-22）

| 判据 | 结果 |
| --- | --- |
| 1 无代理 `--test request_telemetry` | `ok. 15 passed; 0 failed`（0.36s） |
| 1 带 7897 代理 `--test request_telemetry` | `ok. 15 passed; 0 failed`（0.38s）；修前同一命令 `14 passed; 1 failed` |
| 2 `--test launcher` / `--test relay_config` / `--test model_catalog` / `--lib` | 67/0（4.06s）、117/0（0.93s）、20/0（0.46s）、568/0（16 ignored，14.39s） |
| 3 远端仍走代理对拍 | `PROBE local=false -> HTTP status 502 (proxy was used)`、`PROBE local=true -> transport error as expected (proxy bypassed)`，1 passed；探针文件已删除 |
| 4 判定函数边界单测 | `--lib url_target_is_loopback` → `test http_client::tests::url_target_is_loopback_covers_loopback_hosts_and_rejects_others ... ok`，`1 passed; 0 failed; 583 filtered out`；边界 9 条（true: `localhost:57321`、`127.0.0.1:8080`、`127.9.9.9:1`、`[::1]:57321`；false: `10.0.0.5`、`192.168.1.20`、`api.toporeduce.cn`、空串、`not a url`）全通过 |
| 5 改动面 | `cargo fmt --all -- --check` = 0；改动 3 文件（`http_client.rs` / `protocol_proxy.rs` / `model_catalog.rs`）；非 relay 调用点 0 改动；`[DEBUG]` 残留 0 处 |
| 6 模型发现路径（带代理） | `HTTP_PROXY=HTTPS_PROXY=ALL_PROXY=http://127.0.0.1:7897 cargo test -p claude-codex-pro-core --test model_catalog` → `20 passed; 0 failed`（0.46s）；无代理同命令同样 20/0。代理存活证明：`curl -x http://127.0.0.1:7897 https://api.toporeduce.cn/v1/models` → `HTTP 401`（说明请求确实经过了会应答的代理） |
| 7 无 `[DEBUG]` 残留 | `grep -c 'DEBUG' crates/claude-codex-pro-core/src/protocol_proxy.rs` = `0`（原 5 处 `eprintln!` 已删）。该函数为 `normalize_responses_tool_output_call_ids`：仅删除打印与因此不再需要的 `id.clone()`，`call_id` 归一化逻辑未改；由其覆盖用例与全量 `--test relay_config`/workspace 全绿佐证 |
| 附带（本次裁决一并验证） | `request_telemetry` 带 7897 代理 `15 passed; 0 failed`（0.37s，含曾失败的 `helper_handlers_record_real_http_and_preserve_streams`）；`cargo test --workspace --no-fail-fast` = 43 个目标全 `ok`、`WS_EXIT=0`；`npm run check` = 0；`vite:build` ✓ 3.72s；`cargo build --release` = 0（产物 27,949,568 字节 @ 2026-10-22 18:23:35） |

定性结论（供复核）：失败**不是**测试期望过期，也**不是**生产漏记 —— `upstream()` 在核心里写的 `http_status` 正确反映了“真实收到的上游状态码”；`None`/`Some(502)` 的差异来自环境代理是否截胡环回。修在真因侧（环回直连），未放宽任何断言。

## 已知非目标

- 私网段（`10.`、`192.168.`）绕过未做（本机 relay 场景未证明会走这些网段）。
- `crates/claude-codex-pro-core/src/settings.rs:4714` 的 `unused_mut` 与 `tests/uia_real_operation.rs:31` 的既有 warning 未处理（预先存在，非本次引入）。
- 前端死 CSS 清理不属本规格，记录在 `spec/codex-native-model-catalog-from-supplier-mapping.md`。
- `styles.css.bak`（2026-10-04 起入库的既有备份文件）未处理，非本次引入。
