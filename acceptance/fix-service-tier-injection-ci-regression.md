# 验收标准：修复 Service Tier 注入 CI 回归

验证对象：`spec/fix-service-tier-injection-ci-regression.md`

1. 测试断言 `start-conversation` 的 `thread/start` 请求经过 `applyCodexRequestOverrides`。
2. 测试仍断言统一入口内部调用 `applyCodexServiceTierRequestOverride`。
3. 不修改 `assets/inject/renderer-inject.js` 的运行时代码。
4. 以下命令通过：

```powershell
cargo test -p claude-codex-pro-core --test cdp_bridge injection_script_exposes_fast_service_tier_control -- --exact --nocapture
cargo test -p claude-codex-pro-core --test cdp_bridge -- --nocapture
cargo fmt --all -- --check
```
