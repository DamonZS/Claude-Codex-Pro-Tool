# 验收标准：盘古记忆经验教训查看与编辑入口

验证对象：`spec/memory-long-term-edit-button.md`

## 验收项

1. 规格与验收文档存在
   - 通过标准：`spec/memory-long-term-edit-button.md` 与本文档存在。
   - 证据：git diff 或文件检查。

2. 概览页存在经验教训入口
   - 通过标准：盘古记忆总览中“经验教训”旁存在“查看/编辑”按钮。
   - 通过标准：点击按钮会在概览页右侧详情区域展示一条精简经验教训手册，并刷新盘古记忆数据。
   - 通过标准：经验教训入口、详情标题和列表标题不展示“X 条”条数文案。
   - 通过标准：盘古记忆总览信息区使用矩阵布局，操作按钮紧凑排列。
   - 通过标准：盘古记忆主界面不展示“待确认”状态行、待确认列表或确认/忽略候选按钮。
   - 证据：前端源码断言或手动检查。

3. 经验教训手册可查看
   - 通过标准：概览页右侧详情区域展示当前已加载的一条经验教训手册。
   - 通过标准：提炼后 `memory_items` 中最多保留一条 `lesson-manual` 手册记录。
   - 通过标准：手册内容为精简条目，不把每个历史片段分别展示成很多条卡片。
   - 证据：前端源码断言或界面检查。

4. 经验教训可提炼
   - 通过标准：盘古记忆总览、概览右侧详情和盘古记忆管理面板存在“提炼经验教训”按钮。
   - 通过标准：按钮调用真实后端提炼链路，而不是只改前端计数或展示。
   - 通过标准：点击后立即显示运行态反馈，明确说明使用 Codex 本地 SQLite、rollout 会话文件和 `memory_assist.sqlite` 遍历工作区与会话。
   - 通过标准：完成后展示结果摘要，至少包含 DB、会话文件、用户消息、采集记录、自动写入经验教训手册、候选和错误数量。
   - 通过标准：执行后刷新盘古记忆状态与列表。
   - 证据：前端源码断言、Tauri 命令调用检查、定向测试或手动验证。

5. 提炼与按钮动作有日志
   - 通过标准：`run_memory_assist_selfcheck` 写入 `manager.memory.selfcheck.start`。
   - 通过标准：成功时写入 `manager.memory.selfcheck.result`，失败时写入 `manager.memory.selfcheck.failed`。
   - 通过标准：前端按钮点击写入 `manager.ui.button.click`，至少包含当前页面和按钮文案摘要。
   - 通过标准：前端通用动作包装器写入 `manager.ui.action.start`、`manager.ui.action.result`、`manager.ui.action.failed`。
   - 证据：源码断言、运行日志或定向测试。

6. 经验教训手册可编辑
   - 通过标准：经验教训手册有编辑入口。
   - 通过标准：进入编辑状态后可修改文本与分类，可保存或取消。
   - 通过标准：文本为空时保存按钮不可用。
   - 证据：前端源码断言、类型检查或手动检查。

7. 编辑写入使用真实后端命令
   - 通过标准：前端调用 `update_memory_assist_item`。
   - 通过标准：保存请求保留原有 workspace、tags、source、sourceSessionId。
   - 通过标准：保存后刷新盘古记忆列表。
   - 证据：前端源码断言与 Tauri 命令注册检查。

8. 安全边界
   - 通过标准：不修改 Claude 中文注入。
   - 通过标准：不删除 `memory_assist.sqlite`。
   - 通过标准：按用户要求压缩当前保存项前必须备份数据库，且只替换 `memory_items`。
   - 通过标准：不引入新依赖或无关 UI 重构。
   - 证据：git diff 检查。

9. 当前保存项已压缩为手册
   - 通过标准：压缩前生成 `memory_assist.sqlite` 备份。
   - 通过标准：压缩后 `memory_items` 查询结果为 1。
   - 通过标准：唯一保存项的 `category` 为 `lesson-manual`，正文以“经验教训手册：”开头。
   - 通过标准：采集日志、候选和数据库文件本体不被删除。
   - 证据：备份路径、SQLite 查询结果。

## 建议验证命令

```bash
cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem memory -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo build -p claude-codex-pro-manager --manifest-path Cargo.toml
```

## 不在范围内

- 重新设计盘古记忆 UI。
- 改写经验教训提取、检索或自动学习算法。
- 修改 Claude 中文注入或 Codex 注入脚本。
