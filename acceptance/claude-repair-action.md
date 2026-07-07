# 验收标准：修复 Claude 入口

验证对象：`spec/claude-repair-action.md`

## 通过 / 失败标准

### A. 「诊断与修复」入口可见

通过：

- 概览页 `OverviewScreen` 的「诊断与修复」卡片中存在按钮文案「修复 Claude」。
- 该按钮与其它诊断修复按钮同级展示。

失败：

- 只在 Claude 中文区域存在还原入口，诊断与修复卡片中没有「修复 Claude」。
- 文案不是「修复 Claude」。

### B. 复用既有还原链路

通过：

- 「修复 Claude」按钮点击调用 `actions.restoreClaudeZhPatch()`。
- `restoreClaudeZhPatch()` 继续调用 `restore_claude_zh_patch`。
- 不新增重复的还原命令或新算法。

失败：

- 按钮调用了空函数、错误 action 或新写的未验证逻辑。
- 绕过现有确认、运行反馈或错误处理。

### C. 安全边界

通过：

- 不删除 Claude 账号、缓存、数据库或用户数据。
- 不修改 Codex 修复、启动、注入逻辑。
- 不改动 Claude 中文安装逻辑。

失败：

- 该任务引入数据清理、Codex 进程处理或无关 UI 重构。

### D. 测试与构建

通过：

- Rust 结构测试覆盖「修复 Claude」按钮和 `restoreClaudeZhPatch` 调用锚点。
- 前端类型检查通过。
- 前端构建通过。
- manager debug 构建通过，并能报告 `target/debug/claude-codex-pro-manager.exe` 最新时间戳。

失败：

- 没有真实验证结果。
- 构建失败且未说明原因。

## 必需验证命令

至少运行：

```powershell
cargo fmt --check
cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo build -p claude-codex-pro-manager --manifest-path Cargo.toml
git diff --check
```

如 `target/debug/claude-codex-pro-manager.exe` 被占用，可终止 `claude-codex-pro-manager` 进程后重跑构建；不得终止 Codex 进程。

## 完成证据

- 修改文件列表。
- 测试/构建命令输出结果。
- `target/debug/claude-codex-pro-manager.exe` 时间戳。
- 对照本验收标准逐项说明。
