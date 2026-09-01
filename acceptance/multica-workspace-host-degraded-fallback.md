# 验收：Multica 工作区 Host 降级回退

对应规格：`spec/multica-workspace-host-degraded-fallback.md`

## 通过标准

1. `cargo test -p claude-codex-pro-launcher --lib -- --nocapture` 通过。
2. `cargo fmt --check` 通过。
3. Codex Host 不可用时，日志包含
   `multica.workspace_bootstrap_host_fallback`，工作区仍可打开并展示本地
   任务/原生只读库存；执行按钮保持不可用而不是伪造成功。

## 非目标

Host 恢复和真实执行链路需通过 Codex 页面重注入及手动 UI/CDP 验证，
不由本验收文档替代。
