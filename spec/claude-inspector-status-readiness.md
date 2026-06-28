# Claude Inspector 状态就绪判断修复

## 背景

概览页 Claude 状态显示 `Inspector 在线`，但用户点击“修复前端连接”后，详情显示 `127.0.0.1:9229/json` 连接被拒绝。现场进程命令行包含 `--inspect=127.0.0.1:9229`，但系统监听端口中没有 9229，说明当前逻辑把命令行声明误判成了真实在线端口。

## 目标

- 只有 Node Inspector HTTP 端点真实可连接时，才显示 `Inspector 在线`。
- 仅观察到 `--inspect` 命令行但端口未监听时，应显示未验证/受阻，不进入 Node Inspector 注入流程。
- 修复前端连接失败详情必须说明 Inspector 端口未就绪，而不是只输出连接拒绝堆栈。

## 非目标

- 不重新设计 Claude 注入架构。
- 不修改 Claude Desktop 官方文件。
- 不自动杀 Claude 进程。

## 功能要求

- `inspector_ports` 只保存已通过 `127.0.0.1:<port>/json/version` 验证的端口。
- 命令行里观察到的 `--inspect` 端口如果未响应，必须保留到 `debug_evidence` 作为诊断证据。
- `cdp_status` 只有在 `inspector_ports` 非空时才允许为 `node_inspector_ready`。
- 修复前端连接在无可用 CDP/Inspector 时，返回中文可操作详情。

## 验收范围

- `crates/claude-codex-pro-core/src/claude_desktop.rs`
- `apps/claude-codex-pro-manager/src-tauri/src/commands.rs`
- 对应单元/结构测试
