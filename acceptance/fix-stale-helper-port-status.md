# 验收：修复后端端口状态陈旧误报“后端离线”

验证规格：`spec/fix-stale-helper-port-status.md`。

## 通过标准

1. **自愈回退逻辑**（纯函数单元测试）：
   - 记录端口在线 → 采用记录端口，`online=true`，不回退（即使默认端口也在线）。
   - 记录端口离线、默认端口在线、二者端口号不同 → 采用默认端口，`online=true`。
   - 记录端口离线、默认端口离线 → 保持记录端口，`online=false`。
   - `helper_port` 为 None、默认端口在线 → 采用默认端口，`online=true`。
   - 记录端口==默认端口且离线 → 不产生虚假在线（`online=false`）。
2. **debug/frontend 不受影响**：`refresh_launch_port_status` 对 `debug_port_online` 与 `frontend_runtime_online` 的计算与改动前一致。
3. **契约测试不回归**：`apps/claude-codex-pro-manager/src-tauri/tests/windows_subsystem.rs` 全绿（其断言 `refresh_launch_port_status`、`helper_port_online`、`default_helper_port` 等存在）。
4. **构建通过**：`cargo build -p claude-codex-pro-manager` 成功。

## 必需验证方式

- `cargo test -p claude-codex-pro-manager`（含新纯函数测试与 windows_subsystem 契约测试）。
- `cargo build -p claude-codex-pro-manager --release` 或 debug 构建。
- 真机佐证（已采集，作为根因证据）：`latest-status.json` 记录端口 CLOSED、默认端口 57321 `POST /backend/status` 返回 `status:ok`；修复后 overview 应显示“后端在线”。

## 非目标

- 不验证 helper 端口选取/绑定策略。
- 不验证前端渲染像素级表现。
- 不改动 `/backend/status` 协议本身。
