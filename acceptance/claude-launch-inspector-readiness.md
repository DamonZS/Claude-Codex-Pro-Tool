# 验收标准：Claude 启动后调试端口就绪反馈

验证对象：`spec/claude-launch-inspector-readiness.md`

## 通过/失败标准

1. 规格与验收文档存在
   - 通过：本文件和 `spec/claude-launch-inspector-readiness.md` 存在。

2. 启动后必须真实探测
   - 通过：源码包含启动后等待 Claude 进程和调试端口状态的 helper，`open_claude_desktop` 在启动成功后调用它。

3. 未就绪不能伪成功
   - 通过：当 Claude 已启动但 Inspector/CDP 未验证时，返回 `warning` 或失败类状态，并包含中文可操作提示。

4. 成功必须基于真实端口
   - 通过：只有 `node_inspector_ready` 或 `debug_ports` 非空时，启动返回成功类文案。

5. 修复入口保持真实反馈
   - 通过：修复前端连接在没有可用调试端口时不尝试注入，并输出中文原因。

6. 验证命令通过
   - 通过：运行 `cargo fmt --check`、相关 Rust 测试、`npm --prefix apps/claude-codex-pro-manager run check` 和 `git diff --check`。

## 证据要求

- 提供修改文件列表。
- 提供验证命令及结果。
- 如现场 Claude 官方 MSIX 仍不开放端口，必须说明这是外部应用限制，管理工具已改为真实反馈而非误报。
