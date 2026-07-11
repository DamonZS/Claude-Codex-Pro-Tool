# 修复 Service Tier 注入 CI 回归

## 背景

`cdp_bridge` 的 Fast service-tier 注入回归测试仍断言旧的直接调用入口；当前注入脚本已经将 service-tier 与 model selection 合并到 `applyCodexRequestOverrides`，导致 GitHub 自动构建失败。

## 目标

- 更新回归测试以匹配当前统一 override 调用链。
- 继续证明 `thread/start` 会经过统一入口，且统一入口内部仍调用 service-tier override。
- 不改变运行时注入行为。

## 非目标与约束

- 不修改 service-tier、模型选择或 dispatcher 的生产逻辑。
- 不新增依赖，不改发布配置。

## 交付范围

- 更新 `crates/claude-codex-pro-core/tests/cdp_bridge.rs`。
- 新增对应验收标准并运行定向及完整 `cdp_bridge` 测试。
