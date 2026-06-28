# 验收标准：Claude Inspector 状态就绪判断修复

验证对象：`spec/claude-inspector-status-readiness.md`

## 验收项

1. 规格和验收文档存在
   - 通过标准：本文件和 `spec/claude-inspector-status-readiness.md` 存在。

2. 命令行声明不能直接等于在线
   - 通过标准：测试覆盖 `--inspect=127.0.0.1:9229` 存在但 readiness 回调返回 false 时，`inspector_ports` 为空，`debug_evidence` 包含端口未响应说明。

3. 已验证端口才显示 Inspector 在线
   - 通过标准：测试覆盖 readiness 回调返回 true 时，`inspector_ports` 包含该端口。

4. 修复详情有中文可操作反馈
   - 通过标准：源码包含 `Inspector 端口已声明但未就绪` 这类提示，避免只暴露底层连接拒绝错误。

5. 构建和测试通过
   - 通过标准：运行 `cargo fmt --check`、相关 Rust 测试、`npm --prefix apps/claude-codex-pro-manager run check`、`cargo build -p claude-codex-pro-manager`。
