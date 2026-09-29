# 项目构建与临时产物清理验收

对应规格：`spec/project-artifact-cleanup.md`

## 通过标准

1. `target/debug`、`target-rebuild`、`target/release-session-delete-refresh` 和列出的临时目录已移除；`.playwright-cli` 中已跟踪的文件保留。
2. `target/release/claude-codex-pro.exe` 与 `target/release/resources/` 保留。
3. 清理前后 Release 可执行文件 SHA-256 一致。
4. 清理前后 Git 工作区源码修改集合一致。
5. 查询当前 Release 进程状态；若进程已退出，不将文件存在等同于运行验证。
6. 清理后目录占用明显下降，并留下命令输出作为证据。

## 验证方式

- PowerShell 目录体积统计。
- `Get-FileHash`。
- `git status --short`。
- `Get-Process` / `Get-CimInstance Win32_Process`。

## 非目标检查

- 本验收不重新构建应用；后续构建由 Cargo 重新生成清理掉的中间产物。
