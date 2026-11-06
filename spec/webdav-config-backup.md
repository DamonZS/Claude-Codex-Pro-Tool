# WebDAV 配置备份同步（最小切片）

## 背景

CCP 目前只支持把配置备份落在本机目录（`ccp_backup`）。用户在多台机器间使用 CCP 时，需要把同一份备份放到自建 WebDAV（NAS、Nextcloud、坚果云等）上，并在另一台机器上取回。完整的多目录遍历、冲突合并、自动清理不在本次范围内；本次只提供"能上传一份、能按名字取回一份"的最小可用能力，供上层命令与 UI 接线。

本任务对应 cc-switch `commands/webdav_sync.rs` 的最小切片。

## 目标

本次要完成：

- 在 `claude-codex-pro-core` 中提供 WebDAV 备份上传与下载的公开 API。
- 提供稳定的、可排序的备份文件名生成函数（UTC）。
- 对文件名做严格校验，杜绝路径注入。
- 复用仓库既有 HTTP 客户端构造与代理策略，不新建全局客户端单例。
- 为上述行为提供可用本地假 WebDAV 并发执行的集成测试。

本次不包含：

- PROPFIND / 目录遍历 / "列出远端所有备份"。
- 删除、保留策略、增量同步、冲突解决。
- 加密、压缩、分片上传。
- 设置页 UI、Tauri 命令、`commands.rs` / `lib.rs` 接线（由 Lead 按本文件固定签名接线）。
- 修改 `ccp_backup` 的本地备份格式或目录结构。

## 用户视角描述

用户在设置中填入 WebDAV 地址与账号后，点击"上传备份"：工具把当前这份配置备份上传到该目录，并在成功提示中给出远端完整 URL。换一台机器后填入同样的地址、账号和文件名，点击"下载备份"，工具取回同一份字节流并交给既有的备份恢复流程。

失败时用户看到的是一句中文提示：认证失败（401/403）、远端文件不存在（404）或带状态码的其他失败；网络不通时提示里保留底层原因，便于判断是代理、DNS 还是服务端问题。

## 功能要求

### 公开 API（签名固定）

```rust
pub struct WebdavConfig { pub base_url: String, pub username: String, pub password: String }
pub fn backup_file_name(now_ms: i64) -> String   // 形如 ccp-backup-20261106-180500.json（UTC）
pub async fn upload_backup(config: &WebdavConfig, file_name: &str, bytes: &[u8]) -> anyhow::Result<String>
pub async fn download_backup(config: &WebdavConfig, file_name: &str) -> anyhow::Result<Vec<u8>>
```

- `upload_backup` 成功返回远端完整 URL。
- `download_backup` 成功返回响应体字节，内容校验由调用方负责。

### 文件名

- 名称格式：`ccp-backup-YYYYMMDD-HHMMSS.json`，**UTC**，秒精度，毫秒被截断（同秒多次备份同名，以最后一次为准）。
- 只允许 `[A-Za-z0-9._-]` 且非空；同时拒绝整体为点号的名称（`.`、`..`）。
- 违规时返回错误且**不发起任何网络请求**。

### URL 拼接

- `{base_url.trim_end_matches('/')}/{file_name}`；`base_url` 两侧空白先去除，空地址直接报错。
- 文件名先校验再拼接，`../`、`/`、`\` 无法逃出配置目录。

### HTTP 行为

- PUT / GET 使用 `reqwest`，认证方式 `.basic_auth(user, Some(pass))`。
- 单次请求超时 20 秒。
- 上传带 `Content-Type: application/octet-stream`。
- 客户端复用 `crate::http_client::proxied_client_for`，远端目标是回环地址时按既有策略绕过系统代理；不新建全局 `reqwest::Client` 单例。

### 错误文案（中文、带状态码）

- 401 / 403 → 认证失败提示（提示检查用户名和密码），带状态码。
- 404 → 远端文件不存在，带状态码。
- 其他非 2xx → 带状态码的通用失败提示。
- 网络错误 → 使用 `.with_context` 附加上下文，保留 reqwest 原始 cause。

## UI / 交互要求

本次不实现 UI。上层接线时应保证：

- 成功时展示 `upload_backup` 返回的远端 URL 或简洁成功提示。
- 失败时直接展示 API 返回的中文错误文案，不再二次包装成无信息量的"操作失败"。
- 上传/下载进行中应有加载态，且不阻塞其他设置操作（20 秒上限由 API 保证）。

## 数据与接口要求

- 输入：`WebdavConfig`（地址、用户名、密码）、`file_name`、备份字节流。
- 输出：上传为完整 URL `String`；下载为 `Vec<u8>`。
- 不新增持久化结构，不读写任何本地文件，凭据不落盘、不写日志。
- 不做 PROPFIND，因此远端目录必须已存在；目录缺失时由服务端状态码（通常是 404/409）转成中文错误。

## 技术约束

- 只改动 `crates/claude-codex-pro-core/src/webdav_sync.rs`、新增 `crates/claude-codex-pro-core/tests/webdav_sync.rs`、本文件与对应 acceptance 文件。
- 不修改 `commands.rs`、`lib.rs`、`ccp_backup.rs`、前端、`Cargo.toml`、`docs/`、`build-lead/`、`.dsh-ops/`。
- 不新增依赖；测试用 `std::net::TcpListener` 起本地假服务。
- 测试必须能并发执行（随机端口），不依赖外部网络与固定端口。

## 交付范围

- `webdav_sync.rs` 的四个公开 API 与私有校验/错误映射辅助函数。
- `tests/webdav_sync.rs`：PUT 方法与路径/Basic 认证头/字节体、GET 200 字节、GET 404 文案、非法文件名不发请求，以及文件名 UTC 语义。
- 本规格与 `acceptance/webdav-config-backup.md`。