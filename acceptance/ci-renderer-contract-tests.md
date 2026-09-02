# CI 渲染器契约测试同步验收

对应规格：`spec/ci-renderer-contract-tests.md`

## 通过标准

- `codex_multica_workspace_renders_my_issues_as_direct_seven_column_board` 通过，并验证编辑保存使用 `expectedRevision: editor.expectedRevision`。
- `codex_multica_autopilot_manual_trigger_uses_control_plane_run_endpoint` 通过，并验证 `/multica/autopilots/trigger`、`source: "manual"` 和成功提示。
- `cargo test --workspace` 返回退出码 0。

## 验证方式

```powershell
cargo test -p claude-codex-pro-core --test cdp_bridge -- --nocapture
cargo test --workspace
```

## 非目标

GitHub Actions 的 Node.js 弃用提示不属于本次失败标准；自动发布流程由现有 `auto-release-installers` 工作流验证。
