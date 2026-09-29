# 项目构建与临时产物清理记录

对应规格：`spec/project-artifact-cleanup.md`。验收：`acceptance/project-artifact-cleanup.md`。

## 清理前

- 项目 `target/`：97.135 GiB，78,376 个文件。
- `target/debug/`：88.52 GiB；其中 `deps/` 44.70 GiB、`incremental/` 41.20 GiB。
- `target/release/`：5.70 GiB。
- `target/release-session-delete-refresh/`：2.56 GiB。
- `target-rebuild/`：1.91 GiB。
- 当前 Release：`D:\Project\Claude-Codex-Pro-Tool\target\release\claude-codex-pro.exe`，90,503,680 字节，SHA-256 `BB00D29A99307A0A036EEA91C862F209F61CB42302C5091B3042FF8E80A94859`。
- 清理前 PID 11148 正运行上述 Release 文件。

## 清理操作

- 删除 `target/debug/`、`target-rebuild/`、`target/release-session-delete-refresh/`。
- 删除 `target/release/` 下的 `deps/`、`build/`、`.fingerprint/`、`incremental/`、`examples/`。
- 删除 `apps/claude-codex-pro-manager/dist/`、7 个 `.tmp-edge-*` 预览目录及 2 个 `.tmp-index-*` 文件。
- 清理 `.playwright-cli/` 后发现其中有已跟踪历史快照，立即从 `HEAD` 恢复；最终 `git diff --exit-code -- .playwright-cli` 退出 0，且其 `git status --short` 为空。未跟踪的临时截图与快照已移除。

## 清理后验证

- 项目总占用：4.501 GiB，133,847 个文件。
- `target/`：1.274 GiB，11,631 个文件；相对清理前减少 95.861 GiB。
- `target/release/`：0.92 GiB，39 个文件；可执行文件、MCP 程序和 `resources/` 均存在。
- Release SHA-256 仍为 `BB00D29A99307A0A036EEA91C862F209F61CB42302C5091B3042FF8E80A94859`。
- `target/debug/`、`target-rebuild/`、`target/release-session-delete-refresh/`、7 个 `.tmp-edge-*` 目录、2 个 `.tmp-index-*` 文件、前端 `dist/` 均不存在。
- `git diff --exit-code -- .playwright-cli`：退出 0；原有源码修改保留。
- 清理后 `Get-Process -Id 11148` 与按项目路径查询进程均未返回结果。清理操作没有主动结束进程，但当前运行态未通过验收；此记录不宣称应用仍在运行。

## 保留项

- `.git/`、`.codegraph/`、源码、`spec/`、`acceptance/`、AITRACKER 资源、`docs/task-evidence/`、`docs/verification/`、`artifacts/`。
- `target/release/claude-codex-pro.exe` 及其资源和 MCP 程序。
- `target/agent-handoffs/` 等可能包含参考资料的非纯构建目录。
