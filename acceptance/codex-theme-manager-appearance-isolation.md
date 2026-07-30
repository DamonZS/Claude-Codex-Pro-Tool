# 验收标准：Codex 主题与 Manager 外观隔离

验证对象：`spec/codex-theme-manager-appearance-isolation.md`

## 验收项

1. 应用含背景资产的 Codex 主题后，Manager 背景返回空 `data_uri`、空 `source_variable` 和 `user_override = false`。
2. 用户主动选择 CCP 背景后，仍返回对应 Data URI、尺寸、MIME 和 `user_override = true`。
3. 切换 Codex 主题不改变 Manager 背景选择；清除 Manager 背景不改变 Codex 主题。
4. `cargo test -p claude-codex-pro-core codex_theme -- --nocapture` 通过。
5. Manager 类型检查、前端构建和默认 Release 构建通过。

## 非目标

- 不改变 Codex Renderer 的主题视觉。
- 不删除用户已保存的 CCP 背景。
