# 验收标准：历史会话修复等待活动 Provider Sync

对应规格：`spec/fix-provider-sync-active-lock-wait.md`

## 通过标准

1. 使用临时 Codex home 建立带有效 owner 的新鲜锁，同时触发等待型同步；锁在等待上限内释放后，结果为 `Synced`，会话和 SQLite 按既有逻辑更新。
2. 有效锁持续到等待上限后，结果为 `Skipped`，消息明确包含锁占用原因；原锁目录仍存在，会话和 SQLite 未被修改。
3. 普通 `run_provider_sync` 仍对新鲜锁快速返回 `Skipped`，启动器行为不变。
4. 陈旧锁回收测试继续通过。
5. 非锁原因的 `Skipped` 不进入等待循环。
6. Tauri `sync_providers_now` 使用等待型入口，成功、失败和选择持久化语义保持不变。
7. 前端仍在后端完成前显示运行中通知；等待成功只产生一次成功通知和一次 Codex 重启。
8. 测试只使用临时目录和测试数据，不读取、修改或输出真实会话与凭据。

## 必需验证

- 新增“活动锁释放后自动继续”定向测试。
- 新增“活动锁等待超时且不改数据”定向测试。
- 既有新鲜锁快速跳过和陈旧锁恢复测试。
- Manager 命令契约测试或等价源码契约检查。
- `cargo fmt --check`。
- `cargo build --release`，产物位于默认 `target/release`。

## 非目标检查

- 不要求并行写入 Provider Sync。
- 不要求自动终止持锁进程。
- 不修改 Provider、Profile、API Key 或供应商地址。
