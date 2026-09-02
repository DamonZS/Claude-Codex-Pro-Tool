# 验收：本地 Codex 队列领取与派发

对应规格：`spec/multica-local-codex-queue-dispatch.md`

## 通过标准

1. 已分配 Agent 的 queued binding 可在提供可用 host、正确 revision 和 lease token 时创建一个原生 thread，响应包含同一 binding 和真实 handle，binding 为 `dispatched`。
2. 创建请求的 prompt 只包含 Issue 标题/描述和 Agent instructions，且三者均有稳定可辨认的边界。
3. host 不可用、lease 冲突、revision 冲突、Issue/Agent 缺失或 assignee 不匹配时，host 不被调用，binding 仍为 `queued`，且无残留 lease。
4. 已 `dispatched` 的相同请求为重放，返回原 handle 且 host 调用次数不增加。
5. 任何路径均不得将 binding 标为 `running`，也不得启动或注册新的 Codex runtime。

## 必需证据

- 针对上述成功、不可用、冲突和重放分支的 Rust 定向测试。
- `cargo fmt --check`、`git diff --check`。
- 受影响 crate 的测试通过；交付构建前运行前端检查与 `cargo build --release`。

## 非目标

- 不验收远程 Multica daemon、PostgreSQL/Redis 语义或真实页面网络连通性。
