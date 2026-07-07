# 验收标准：概览刷新反馈

验证对象：`spec/overview-refresh-feedback.md`

## 通过 / 失败标准

### A. 用户点击有即时反馈

通过：

- 「诊断与修复」中的「刷新概览」按钮调用 `actions.refreshRoute("overview", { notify: true })`。
- `refreshRoute` 在 `notify: true` 时先显示运行中 Notice。

失败：

- 按钮仍调用 `actions.refreshRoute("overview")` 且无通知参数。
- 点击后没有任何 Notice。

### B. 完成后有结果反馈

通过：

- 主动刷新完成后显示「概览已刷新。」。
- 后端调用异常时仍由现有 `run` 机制显示失败通知。

失败：

- 刷新完成后仍无完成提示。
- 失败被吞掉且没有通知。

### C. 自动刷新不被打扰

通过：

- 路由切换和 `useEffect` 自动调用 `refreshRoute(route)` 时不传 `notify: true`，仍保持静默。

失败：

- 切换页面时频繁弹出刷新提示。

### D. 验证

通过：

- 结构测试覆盖按钮调用和通知逻辑锚点。
- 前端类型检查通过。
- 前端构建通过。
- manager debug 构建通过，并报告 exe 时间戳。

## 必需验证命令

```powershell
cargo fmt --check
cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo build -p claude-codex-pro-manager --manifest-path Cargo.toml
git diff --check
```

如果 debug manager exe 被占用，只允许终止 `claude-codex-pro-manager` 进程；不得终止 Codex。
