# CCP 本轮交付记录（2026-09-19）

本记录汇总原始三页移植、原生子任务、详情返回、属性管理、盘古记忆移除、请求时间线和标题栏锚点的最终证据。

## 主要修改

- `crates/claude-codex-pro-core/src/multica_workspace/native_subtasks.rs` 从 Codex 本地数据库只读查询真实父子关系和轮次状态；`apps/codex-workflow-surface/src/native-subtasks.tsx` 接入“我的任务”，显示昵称、父会话、状态和更新时间。
- `assets/inject/renderer-inject.js` 使用现有 Codex 原生 opener 打开侧栏中不存在的子会话，并核验精确子标签；`apps/codex-workflow-surface/src/main.tsx` 接通三类详情返回和属性二级路由。
- 固定的原始 PropertiesTab、adapter、Core capability、manifest 和 vendor 闭包接通属性 CRUD、任务属性值及真实视图偏好。
- 保留此前盘古记忆移除、会话删除、三轨时间线加 Token 行和版本号最后一位到缩小按钮的 8px 锚点修改。

## 验证结果

- Manager `npm run check`：通过。
- Manager `npm run vite:build`：通过，包含 264 项工作流测试、25 项注入测试、生产 bundle 挂载检查和两套 Vite 构建。
- `cargo test --workspace -j 1`：退出 0；去除嵌套子进程重复计数后 1521 passed / 10 ignored。Core 704 passed / 7 ignored，Manager 71 passed / 2 ignored。
- `cargo fmt --check`、`git diff --check`、LICENSE/NOTICE 字节核验：通过。
- `cargo build --release -j 1`：退出 0，6m12s。
- 时间线、标题栏、release、原生子任务和返回 verifier 定向回归：通过；返回 verifier 23 项离线测试通过。

## 最终产物

- [默认 Release EXE](D:/Project/Claude-Codex-Pro-Tool/target/release/claude-codex-pro.exe)
- 大小 88,799,232 bytes；SHA-256 `EDA0174FE2EB4FC8300FA6B3E851BFD7E2D99E09BFDE61FD4B160DD512D06F7D`。
- Manager PID 12504 从上述项目路径运行；修复命令返回 `status:ok`、`codexFrontendInjected:true`、`codexBackendOnline:true`。
- 回滚副本：`target/release/claude-codex-pro.before-properties-20260919.exe`。

## 实机证据

- 三页最终 Release live 检查：1004×731，零可见告警；截图在 `acceptance/evidence/multica-live/`。
- 属性：创建数字属性、改名重载、任务值 13→21、归档后保留并只读、点击“未设置”清空成功。视图：真实 `builtin:assigned` 与 `view:<id>` 显隐、鼠标拖排、重载保持成功。证据分别在 `acceptance/evidence/property-live/2026-09-18T17-04-23-199Z/` 和 `acceptance/evidence/property-live/2026-09-18T17-09-15-121Z/report.json`。
- Issue、Autopilot、Agent 三类详情真实返回及 Host 选择同步：`acceptance/evidence/workflow-return-live/2026-09-18T17-09-51-620Z/`，报告 `ok:true`。
- Descartes、Tesla、Faraday、Mendel 四个原生子任务均可见；Descartes 精确子标签打开后恢复父会话：`acceptance/evidence/native-subtasks-live/2026-09-18T17-11-19-783Z/`，报告 `ok:true/restored:true`。
- 最终 DOM：1 行 Token、3 条轨道（Provider 请求/协议与代理/Agent 响应）、盘古记忆标签 0；版本号最后一位距缩小边界 8px。

## 数据保留与边界

- 本轮自建 Issue/view 已清理；测试属性仅归档且 usage_count 为 0；原视图偏好按 revision 校验后恢复。
- `C:/Users/Damon/.claude-codex-pro/memory_assist.sqlite` SHA-256 `720992EB0449DC26B5A4B1A9842FF5DFFC18D5C5933B075F6256963A5101A75A`。
- `F:/pangu/memory_assist.sqlite` SHA-256 `D5F00A98D612FDCDB8FED4765A3DE05CE1FFB0275BC8F3C0F72F9FB1AB76A210`。Git diff 中无 sqlite/wal/shm 文件。
- 可执行源码搜索 `memory_assist|MemoryAssist|盘古记忆|/memory/` 零命中；未写 Codex auth/config。
- 附件/上传、Squad 执行及矩阵列出的部分导入/复制和评论触发能力仍按 `apps/codex-workflow-surface/API_ADAPTER_MATRIX.md` 记录；四个子线程 history 没有 turn 行，因此显示“状态待确认”。
