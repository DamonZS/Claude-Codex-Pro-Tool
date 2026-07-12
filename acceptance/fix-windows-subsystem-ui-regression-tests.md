# 验收标准：修复 Windows Manager UI 回归测试

验证对象：`spec/fix-windows-subsystem-ui-regression-tests.md`

## 通过标准

1. 工具页回归测试锁定当前统一工具资产面板，不再引用已移除的 `ContextManagerPanel`。
2. 插件仓库回归测试匹配当前仓库配置状态文案。
3. 会话上下文的 `role="dialog"` 与 `aria-modal="true"` 保持存在。
4. 通知系统仍不存在旧的 `notice-backdrop`、`notice-card` 模态结构。
5. Manager 的完整 `windows_subsystem` 测试通过。
6. 未修改 Manager 运行时代码或 GitHub Actions 工作流。

## 必需验证

```powershell
cargo test -p claude-codex-pro-manager --test windows_subsystem -- --nocapture
cargo fmt --all -- --check
git diff --check
```

## 完成证据

- 上述命令的退出码与测试摘要。
- `git diff` 显示本任务仅修改测试、规格和验收文档。

## 非目标检查

- 不要求重跑或发布新的 GitHub Release。
- 不要求修改会话上下文或工具页视觉设计。
