# 验收标准：Claude Desktop 代理上游与模型列表修复

验证对象：`spec/claude-desktop-proxy-upstream-models.md`

## 验收项

1. 规格与验收文档存在
   - 通过标准：本文件和 `spec/claude-desktop-proxy-upstream-models.md` 存在。

2. 默认模型列表为四项
   - 通过标准：测试验证空模型列表时，Claude Desktop 模型发现返回 `claude-fable-5`、`claude-haiku-4-5`、`claude-opus-4-8`、`claude-sonnet-4-6`。

3. Profile 写入包含默认四模型
   - 通过标准：测试验证 provider/dev-mode profile 在模型列表为空时仍写入四个 `inferenceModels`。

4. 上游 Base URL 不再为空
   - 通过标准：测试验证 active relay 上游为空时，Claude Desktop 代理解析到 `https://api.toporeduce.cn`，不会因 Base URL 为空直接 502。

5. 真实验证通过
   - 通过标准：至少运行并通过：
     - `cargo fmt --check`
     - `cargo test -p claude-codex-pro-core protocol_proxy -- --nocapture`
     - `cargo test -p claude-codex-pro-core --test claude_desktop_provider -- --nocapture`
     - `cargo test -p claude-codex-pro-manager --test windows_subsystem -- --nocapture`
     - `npm --prefix apps/claude-codex-pro-manager run check`
     - `cargo build -p claude-codex-pro-manager`
