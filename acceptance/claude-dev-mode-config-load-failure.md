# 验收标准：Claude 开发配置加载失败回归修复

验证对象：`spec/claude-dev-mode-config-load-failure.md`

## 验收项

1. 规格文档存在
   - 通过标准：`spec/claude-dev-mode-config-load-failure.md` 存在。
   - 证据：文件存在检查。

2. 验收文档存在
   - 通过标准：`acceptance/claude-dev-mode-config-load-failure.md` 存在。
   - 证据：文件存在检查。

3. 写入 profile 时 `_meta.json` 必须指向该 profile
   - 通过标准：provider 存在但 API Key 为空时，`configure_claude_desktop_dev_mode_with_proxy_port` 仍写入 profile、`entries` 包含本工具 profile、`appliedId` 等于本工具 profile id。
   - 证据：定向 Rust 测试。

4. 空模型列表不写入默认 `inferenceModels`
   - 通过标准：`model_list` 为空时 profile 中不存在 `inferenceModels` 字段。
   - 证据：定向 Rust 测试。

5. 显式模型列表仍可写入安全模型
   - 通过标准：`model_list` 非空时 profile 中包含安全 Claude model id，且可保留 `supports1m` / label 信息。
   - 证据：既有或新增 Rust 测试。

6. 状态检测不再因 API Key 为空误判开发模式未写入
   - 通过标准：正常配置、profile 和 meta 都存在时，即使 API Key 为空，`load_claude_desktop_dev_mode_status` 语义对应“已配置/已写入”。
   - 证据：定向 Rust 测试覆盖底层配置判断。

7. 真实验证通过
   - 通过标准：至少运行以下命令并通过：
     - `cargo fmt --check`
     - `cargo test -p claude-codex-pro-core --test claude_desktop_provider proxy_port -- --nocapture`
     - `cargo test -p claude-codex-pro-core plugin_hub -- --nocapture`
     - `cargo build -p claude-codex-pro-manager`

## 不在范围内

- 不要求在当前回合手动重启用户的 Claude Desktop。
- 不验证真实第三方 API Key 的可用性。
- 不修改 Claude 汉化资源或注入标识。
