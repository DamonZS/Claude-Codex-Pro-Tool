# 项目构建与临时产物清理

## 背景

项目目录中的历史 Rust 增量构建、临时 Release 构建和浏览器预览缓存累计占用大量磁盘空间，影响后续构建与检索。

## 目标

- 删除确认可由 Cargo、Vite 或预览工具重新生成的构建缓存和临时目录。
- 保留当前 `target/release/claude-codex-pro.exe`、其运行所需资源、源码、规格、验收、AITRACKER 资源和验证证据。
- 不改变 Git 工作区中既有源码及用户数据。

## 清理范围

- `target/debug/`
- `target-rebuild/`
- `target/release-session-delete-refresh/`
- `target/release/{deps,build,.fingerprint,incremental,examples}/`
- 根目录 `.tmp-edge-*`、`.tmp-index-*`、`.playwright-cli/` 中未跟踪的临时文件
- `apps/claude-codex-pro-manager/dist/`

## 非目标

- 不删除 `target/release/claude-codex-pro.exe`、`target/release/resources/` 或运行所需的 MCP 可执行文件。
- 不删除 `.git/`、源码、文档、`spec/`、`acceptance/`、`assets/aitracker/`、`docs/task-evidence/`、`docs/verification/`、`artifacts/`。
- 不删除已被 Git 跟踪的 `.playwright-cli/` 历史快照。

## 验证

- 清理前后记录重点目录体积。
- 确认 Release 可执行文件仍存在且哈希不变。
- 确认 Git 工作区源码状态未因清理发生变化。
- 查询当前 Release 进程状态；进程可能由用户或系统在清理期间关闭，结果须如实记录。
