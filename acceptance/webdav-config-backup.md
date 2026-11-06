# 验收标准：WebDAV 配置备份同步（最小切片）

验证对象：`spec/webdav-config-backup.md`

## 验收项

1. 规格与验收文档存在。
   - 通过标准：`spec/webdav-config-backup.md` 与 `acceptance/webdav-config-backup.md` 存在，且验收项与规格目标一一对应。
   - 证据：文件存在检查。

2. 公开 API 与规格签名一致。
   - 通过标准：`crates/claude-codex-pro-core/src/webdav_sync.rs` 导出 `WebdavConfig`、`backup_file_name(now_ms: i64) -> String`、`async upload_backup(&WebdavConfig, &str, &[u8]) -> anyhow::Result<String>`、`async download_backup(&WebdavConfig, &str) -> anyhow::Result<Vec<u8>>`，字段与参数类型未改动。
   - 证据：源码检查；`lib.rs` 中既有 `pub mod webdav_sync;` 未被改动。

3. 文件名规则符合规格。
   - 通过标准：`backup_file_name` 生成 `ccp-backup-YYYYMMDD-HHMMSS.json`，按 UTC 计算，毫秒被截断。
   - 证据：测试 `backup_file_name_uses_utc_second_precision` 通过。

4. 上传请求符合协议要求。
   - 通过标准：本地假 WebDAV 收到 `PUT`、正确路径（含尾斜杠归一）、`Authorization: Basic <base64(user:pass)>`、`Content-Type: application/octet-stream`，字节体与传入一致，服务端 201 时返回完整远端 URL。
   - 证据：测试 `upload_puts_body_with_basic_auth_and_returns_remote_url` 通过。

5. 下载请求符合协议要求。
   - 通过标准：假服务端收到 `GET` 与正确路径，200 时返回的 `Vec<u8>` 与响应体逐字节相等。
   - 证据：测试 `download_returns_bytes_from_get` 通过。

6. 错误文案符合规格。
   - 通过标准：404 错误文案包含"不存在"和状态码；401/403 文案包含"认证失败"和状态码；其它非 2xx 文案包含状态码；网络错误保留 reqwest 原始 cause（`.with_context`）。
   - 证据：测试 `download_404_reports_missing_remote_file`、`upload_401_reports_auth_failure_with_status_code` 通过；`.with_context` 源码检查。

7. 路径注入被拒且不发请求。
   - 通过标准：`../evil.json`、`sub/evil.json`、`sub\evil.json`、`..`、`.`、空名、非 ASCII 名均返回"文件名不合法"，且监听端口未收到任何连接。
   - 证据：测试 `unsafe_file_names_are_rejected_before_any_request` 通过。

8. 复用既有 HTTP 客户端，不新增全局单例。
   - 通过标准：`webdav_sync.rs` 通过 `crate::http_client::proxied_client_for` 构造客户端，回环目标使用 `url_target_is_loopback`，单次请求 20 秒超时；文件内无 `static CLIENT` / `OnceLock<reqwest::Client>`。
   - 证据：源码检查。

9. 集成测试可并发执行且不引新依赖。
   - 通过标准：测试使用 `TcpListener::bind("127.0.0.1:0")` 取随机端口，只使用既有依赖（`reqwest`、`tokio`、`chrono`、`base64`）；`Cargo.toml` 无改动。
   - 证据：测试源码检查；`Cargo.toml` diff 为空。

10. 只改动允许范围内的文件。
    - 通过标准：变更仅限 `crates/claude-codex-pro-core/src/webdav_sync.rs`、`crates/claude-codex-pro-core/tests/webdav_sync.rs`、`spec/webdav-config-backup.md`、`acceptance/webdav-config-backup.md`；未触碰 `commands.rs`、`lib.rs`、`ccp_backup.rs`、前端、`Cargo.toml`、`docs/`、`build-lead/`、`.dsh-ops/`。
    - 证据：`git status --short` 与 diff 检查（由 Lead 或评审代理执行；本任务不执行 git 操作）。

## 必需验证

至少运行（PowerShell）：

```powershell
$env:CARGO_TARGET_DIR="D:\Project\Claude-Codex-Pro-Tool\build-lead"
$env:TMP="D:\Project\Claude-Codex-Pro-Tool\.tmp-build"; $env:TEMP=$env:TMP
cd D:\Project\Claude-Codex-Pro-Tool
cargo test -p claude-codex-pro-core --test webdav_sync
rustfmt --edition 2024 --check crates\claude-codex-pro-core\src\webdav_sync.rs crates\claude-codex-pro-core\tests\webdav_sync.rs
```

通过标准：`cargo test` 输出 `6 passed; 0 failed`，`rustfmt --check` 退出码 0。

## 证据清单

- `cargo test -p claude-codex-pro-core --test webdav_sync` 的完整尾段输出（含 `test result: ok.` 行）。
- `rustfmt --check` 的退出码。
- 四个变更文件的最终内容（源码检查复用的客户端函数与行号）。

## 不在范围内

- `cargo build --release`、安装包构建、应用安装或重启。
- 目录遍历、远端列表、自动清理、冲突合并。
- 设置页 UI、Tauri 命令接线与持久化。
- 任何 git 操作（提交、分支、工作树）。
- 真实第三方 WebDAV 服务的连通性验证（本次仅验证协议行为，网络环境差异由后续联调覆盖）。