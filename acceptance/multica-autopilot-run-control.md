# 验收：Multica Autopilot Run Control

对应规格：`spec/multica-autopilot-run-control.md`

## 通过标准
- Rust store 能创建 `pending` run，并按 autopilot 列表及按 ID 查询。
- 非法 source 被拒绝，超过容量被拒绝。
- bridge 路由返回真实持久化记录，不再生成前端 `unsupported` 临时对象。
- 前端“立即触发”调用 bridge 并刷新自动化资源。

## 验证方式
运行 `cargo fmt --check`、定向 core 测试、`node --check assets/inject/renderer-inject.js`、Manager check/build 和 `cargo build --release`。

## 非目标
未接入远端 scheduler、Webhook delivery worker、Codex runtime 执行的部分不得标记为完成。
