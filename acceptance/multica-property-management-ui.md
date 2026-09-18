# 原始属性管理与视图偏好 UI 验收

对应 [规格](../spec/multica-property-management-ui.md)。

- [x] 我的任务点击进入原始 PropertiesTab；返回及直接深链加载/错误返回有效，仍只有三个顶级模块。
- [x] 本地能力 true 时 member 可管理；false/缺失时无管理按钮，身份不提升。
- [x] 原始 UI 创建属性、编辑名称/选项、归档、显示归档及恢复的真实点击写入正确 Core fixture。
- [x] 原始 Issue 编辑器设置/清空值保留无关字段；重挂载恢复写入值。
- [x] 真实管理视图点击使用 builtin:/view: ID，排序/隐藏与重挂载读取一致。
- [x] 属性查询/写入失败可见，无假空成功；adapter CAS/非法类型回归通过（Core 重跑归父线程）。
- [x] 固定 Git 对象生成闭包；source-files.json 原始/派生哈希通过验证，署名保留。
- [x] TypeScript 与定向 Vitest 通过；相关既有 main/adapter 测试通过。
- [x] 父线程统一前端 build → Rust → 默认 Release，记录独立结果。
- [x] Release 真实 UI 创建 CCP验收 临时属性/Issue，编辑/归档/清空/重载；仅清理记录的自建 ID。
  视图偏好先保存原值，恢复前核对并发变化。此项独立于 fixture 测试。

## 证据

2026-09-19 最终 Release 实机闭环：

- `cargo build --release -j 1` 退出 0（6m12s）。默认 EXE 为 88,799,232 bytes，
  SHA-256 `EDA0174FE2EB4FC8300FA6B3E851BFD7E2D99E09BFDE61FD4B160DD512D06F7D`。
  Manager PID 12504 从项目默认路径启动；实际 Tauri 修复命令确认
  `status:ok/codexFrontendInjected:true/codexBackendOnline:true`。
- 原始 PropertiesTab 点击创建 number 属性、编辑名称、切页重载通过；原始 Issue
  控件把值 13 保存，重载后改为 21；归档定义后值仍保留，编辑入口只读，点击
  “未设置”清空成功。数据均通过真实 Core 查询读回。
  证据 `evidence/property-live/2026-09-18T17-04-23-199Z/` 的四项 checks 与截图。
  该次脚本在后续创建视图 fixture 时带了仅适用于 Issue 的 suppressRun，被 Core
  正确拒绝；这不影响前述已完成的属性 checks。随后仅修正临时验证脚本。
- 视图专项 `evidence/property-live/2026-09-18T17-09-15-121Z/report.json` 为
  `ok:true`：原始管理视图弹窗操作 builtin:assigned 与 view:<真实自建ID> 显隐；
  实际鼠标拖拽自建视图到第二位，Core order 精确匹配；切页重载仍保持。
  直接创建 Core fixture 后先使用现有公开 invalidate 入口刷新页面缓存。
- 最终测试视图 `ccp-acceptance-view-mu77qvx7` 已删除；偏好恢复前核验本次最后
  revision/prefs 并以 CAS 恢复。各次自建 Issue/view 已按返回 ID 删除，测试属性
  仅归档保留，未更改用户原有任务、属性值或视图配置。
- 三页截图由最终内嵌 Release 注入后生成，尺寸 1004×731、零可见告警。
  此处为真实运行验证，与下面的 DOM/Core fixture 测试分别记录。

2026-09-19 父线程统一验证：

- Manager `npm run check` 通过；`npm run vite:build` 完整流水线退出 0：
  264 项工作流测试、25 项注入测试、构建产物离线挂载测试及两套 Vite 构建通过。
- 固定前端产物后运行 `cargo test --workspace -j 1` 退出 0：Core 单元测试
  704 passed / 7 ignored，Manager 单元测试 71 passed / 2 ignored，全部集成、
  launcher/data 及文档测试通过。去除六次嵌套子进程重复统计，共 1521 passed / 10 ignored。
- `cargo fmt --check`、`git diff --check` 与发布 LICENSE/NOTICE 字节核验通过。
- 日志：`%TEMP%/ccp-final-property-frontend.log`、`%TEMP%/ccp-final-stable-workspace.log`。
  默认 Release 构建及真实属性持久化验收已在本记录前段单独记录。

2026-09-19 源码稳定点：

- `npm --prefix apps/codex-workflow-surface run vendor`：从固定 Git 对象生成 824 文件。
- `npm --prefix apps/codex-workflow-surface test -- src/property-management-ui.test.tsx src/main.test.tsx src/native-subtasks.test.tsx src/multica-api-adapter.test.ts src/multica-adapter-query.test.ts src/source-manifest.test.ts`：6 文件、191 项通过，无未处理错误。
- 测试查询 TypeScript 参数修正后，`npm --prefix apps/codex-workflow-surface run check` 退出 0；
  `npm --prefix apps/codex-workflow-surface test -- src/property-management-ui.test.tsx` 重跑 9 项通过。
- 修改范围 `git diff --check` 退出 0。
- 本轮 Core 仅新增 bootstrap permissions 类型/字段和由 enabled 赋值的构造；不改 routes、数据库或执行策略。
  前端 fixture 验证能力投影及 member 身份保留；Core 编译/测试由父线程统一执行。
- 上述是原始组件 DOM + adapter + 内存 Core 契约 fixture 验证，不替代 Release 实机持久化。
  按父线程构建顺序，本轮未运行 Vite build、Cargo 或操作当前 UI，dist 未生成。
