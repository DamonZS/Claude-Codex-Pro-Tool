# 验收标准：Codex DevTools 目标隔离

验证对象：`spec/codex-devtools-target-isolation.md`

## 验收项

1. Claude 页面不被选中
   - 通过标准：CDP 列表仅包含 `title=Claude`、`url=app://localhost/epitaxy` 且有 WebSocket 的 page 时，严格 Codex 选择器返回 `No injectable Codex page target found`。
   - 证据：`cdp_bridge` 定向 Rust 测试。

2. Launcher 不允许任意页面回退
   - 通过标准：`LauncherRuntimeService::open_devtools` 使用严格 Codex 页面选择器；找不到 Codex target 时在构造 DevTools URL 前失败，因此不会调用系统 URL 打开器。
   - 证据：源码审查和定向编译测试。

3. 回归边界
   - 通过标准：Codex target 仍可由严格选择器识别；不修改 Claude 显式启动/调试路径、任务工作区或注入 UI。
   - 证据：现有 `cdp_bridge` Codex 识别测试和定向 Rust 测试。

## 非验收范围

- 不从当前用户机器启动、关闭或重启 Claude Desktop。
- 不修改 Codex 或 Claude 原生安装文件。
