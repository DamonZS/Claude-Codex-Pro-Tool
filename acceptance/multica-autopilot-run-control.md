# 验收：Multica Autopilot Run Control

对应规格：`spec/multica-autopilot-run-control.md`

## 通过标准
- Rust store 能创建 `pending` run，并按 autopilot 列表及按 ID 查询。
- 手动触发真实存在且 active 的 autopilot 后，`create_issue` 生成带来源标记的 Issue 与 execution binding；`run_only` 直接生成 binding。
- 无 Codex host 时不得报告 completed，必须保留 queued/pending 和稳定诊断码；重复触发幂等。
- `run_only` 触发前后工作区 Issue 集合数量和内容不变，运行记录进入 `running` 或在 host 不可用时保持可诊断的 queued/pending。
- 非法 source 被拒绝，超过容量被拒绝。
- bridge 路由返回真实持久化记录，不再生成前端 `unsupported` 临时对象。
- 前端“立即触发”调用 bridge 并刷新自动化资源。

## 验证方式
运行 `cargo fmt --check`、定向 core 测试、`node --check assets/inject/renderer-inject.js`、Manager check/build 和 `cargo build --release`。

## 非目标
未接入远端 scheduler、Webhook delivery worker、Codex runtime 执行的部分不得标记为完成。
