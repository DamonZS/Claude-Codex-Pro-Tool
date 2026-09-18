# 验收：我的任务中的 Codex 原生子任务

对应 `spec/codex-native-subtasks-in-my-tasks.md`。

- [x] 真实 parent/child 行生成唯一只读条目，普通线程不混入。
- [x] 名称优先昵称，不展示完整任务指令；状态由最新 turn 确认，open/closed 不冒充 running/completed。
- [x] 原有 Issue 看板、筛选、详情返回正常；区域默认可见，数据可刷新。
- [x] 打开子/父会话调用现有 Host 入口，失败显式展示；无新建执行副作用。
- [x] 加载、空、错误、过期与重试通过行为测试；分页有界。
- [x] Rust/前端定向测试、类型检查、构建通过。
- [x] 默认 Release 重新注入后截图里的四个原生子线程可见，保存脱敏证据。

## 证据

最终属性版本回归：2026-09-19 01:11，默认 Release SHA-256
`EDA0174FE2EB4FC8300FA6B3E851BFD7E2D99E09BFDE61FD4B160DD512D06F7D`，
Manager 实际修复后 generation 21。原生子任务 live 再次退出 0，四个 ID/昵称可见、
子标签精确选中、父会话恢复，`ok:true/restored:true`。
证据：`evidence/native-subtasks-live/2026-09-18T17-11-19-783Z/`。

### 默认 Release 实机验证（2026-09-19 00:40 Asia/Shanghai）

- Manager 使用默认 Release 的内嵌资产重新注入；三页 live 检查均通过，页面尺寸
  1004×731，零可见告警。此版本 SHA-256 为
  `6B4E39CEAE4CC5D0F007B8E13FD8DAF5FB3417794AFD4200BF15766F17528237`。
- `node scripts/verify-native-subtasks-live.mjs` 退出 0：四个真实 ID/昵称均可见；
  Descartes 打开后精确 `background-agent:<id>` 标签被选中；随后父会话恢复，
  仅本次创建的 UI 子标签被关闭，`restored:true`。
- 脱敏截图与 JSON：`evidence/native-subtasks-live/2026-09-18T16-40-56-170Z/`。
- 四个子线程在原生 history 中均无 turn 行，故显示“状态待确认”；schema 与
  `inProgress/completed/failed/interrupted` 解析已只读核实，open 边未误报完成。
- 属性管理最终集成后的统一 Release 验证另见该任务验收记录。

此任务不将原生线程转成可编辑的 CCP Issue，也不替换 Codex 执行器。

### 前端验证（2026-09-19）

- `npm --prefix apps/codex-workflow-surface test -- native-subtasks.test.tsx main.test.tsx`：
  **46 passed**。覆盖四个真实形状子任务与原页面共存、Host mount 打开回调优先级、
  子/父 ID 跳转、加载/空/错误、部分读取保留完整旧数据、恢复刷新、卸载停止轮询。
- Workflow 与 Manager `npm run check` 通过。Manager `npm run vite:build` 完整流程
  通过（248 个 workflow 测试、17 个注入测试、源码清单、两套 Vite 产物）。
  随后的 partial-read/mount 修正已跑上述定向回归并重新通过 workflow check/build，
  `node scripts/test-workflow-artifact.cjs` 生产 bundle 挂载/卸载验证通过。
- `cargo fmt --check` 与盘古记忆可执行源码残留检查通过；Git 数据库文件差异为空。
- 实机跳转仍须使用最终 Release 验证；fixture 通过不代替原生控件点击证据。

### Core 定向验证（2026-09-19）

- `cargo check -p claude-codex-pro-core --lib -j 1`：通过。
- `cargo test -p claude-codex-pro-core --lib native_subtasks -j 1 -- --test-threads=1`：
  **7 passed, 0 failed**（699 filtered out，测试耗时 7.67 秒）。此前路由测试的
  `Value` 导入编译问题经主线程修正后，本次已实际执行全部七项。
- SQLite fixture 分别建立 session 与 `thread_history_1.sqlite` 数据库，覆盖真实
  父子边、普通线程排除、昵称/短 ID、标题不输出、跨库去重、每页 100 与近期 500
  上限，以及缺库、损坏、schema 不兼容时的错误或 `stale`。
- history fixture 使用核实的 `thread_turns` 列与复合主键；最新轮次按
  `rollout_ordinal` 排序，即使 turn ID、started_at 顺序相反也不改变判定。
  覆盖 `inProgress/completed/failed/interrupted`、无轮次及未知值；父子边
  open/closed 与归档标记均不决定执行状态。
- 正常查询前后 session/history fixture 数据库字节一致；缺失数据库未被创建，
  generic workspace upsert 拒绝该资源。测试未写入真实用户数据库。
- 对 `multica_workspace.rs`、`multica_workspace/native_subtasks.rs` 和
  `multica_workspace/native_subtasks_tests.rs` 运行
  `rustfmt --check --edition 2024 --config skip_children=true`：通过。
- ALL 枚举顺序回归：直接运行本次构建的
  `target/debug/deps/claude_codex_pro_core-c17486993529f452.exe --exact multica_workspace::tests::module_order_is_fixed_and_contains_skills`，
  **1 passed, 0 failed**。原排队 Cargo 单项命令已取消，避免继续占用共享构建队列。
- bootstrap 的 ALL 查询之后仍由旧 inventory 覆盖同名 `codex_native_agents`，
  按主线程要求保持兼容。已只读核对 `native-subtasks.tsx` 直接调用
  `/multica/workspace/query`，独立区域消费新资源，不以该 bootstrap 条目初始化。
- 仅有已有 `install/macos.rs` 的 `LEGACY_MANAGER_NAME` unused-import 警告。

上述证据仅覆盖 Core 资源及静态接线；前端行为测试、完整构建、真实 UI 与 Release
验收由主线程记录，不据此勾选整体验收项。

### 原生子会话打开路径审计与回归（2026-09-19）

- 当前已打开的 Codex 父线程为 `01a0b2ac-7535-7481-a638-70d793805812`。
  以下四个子 ID 均不在 `[data-app-action-sidebar-thread-id]` 集合中，但父会话
  已挂载的原生 `backgroundAgentOpener.canOpen(id)` 均返回 true：
  Descartes `01a0b405-80ed-7662-b24d-5c22329d64ac`；
  Tesla `01a0b405-81bf-7401-a1c2-18c0dddc5ff7`；
  Faraday `01a0b405-8314-7503-8170-ff860eb0aa6c`；
  Mendel `01a0b405-8422-7582-b796-4cff65cd2e8a`。
- 审计当前页面实际加载的 `app://-/assets/` 原生代码：
  `conversation-blocks-ff7713443848.js` 的 multi-agent 链接仅在
  `canOpen(agentId) === true` 时提供按钮，回调为 `open(agentId, displayName)`；
  `local-conversation-page-e50b4ba5d33e.js` 按 canInteract 选择 background-agent
  或 subagents 原生面板；`open-local-conversation-background-agent-607554448ba6.js`
  使用 `background-agent:{conversationId}` 标签及真实 conversationId；
  `open-local-conversation-subagents-panel-a4be53a48e39.js` 对后代关系与加载状态
  进行核验，并保留 requestedConversationId/isLoading。未复制原生 bundle。
- 在旧 Release 中仅点击现有 Descartes 原生按钮，确认选中标签
  `data-tab-id="background-agent:01a0b405-80ed-7662-b24d-5c22329d64ac"`、
  `aria-selected="true"`；其父会话仍为侧栏 active。因此侧栏 active 不是
  子会话面板打开的充分判据。检查后关闭本次新建的 UI 标签并点击原父线程；
  最终 CDP 确认 parentActive=true、probeTabPresent=false、bridge=true。
  本次仅导航已有线程，未创建线程、未发消息、未注入 raw renderer。
- renderer 保留原顶层侧栏/折叠项目路径；缺行时核验 thread/read ID，再扫描
  最多 200 个原生消息按钮、每个最多 16 层已挂载 Fiber 获取已有 opener。
  仅 canOpen=true 时调用该原生回调；等待最多 3 秒确认精确标签/已加载子面板。
  回调返回或父线程 active 均不单独算成功，失败保留工作区。
- `node --test scripts/test-workflow-native-open.cjs scripts/verify-native-subtasks-live.test.mjs`：
  **19 passed, 0 failed**。覆盖原顶层打开、折叠项目、ID 不匹配、缺少子侧栏行、
  原生 capability/入口缺失、错误子标签、尚未激活或仍加载的面板，以及 live
  verifier 的子标签证据、父线程恢复与仅关闭本次新标签。
- renderer 和 live verifier 的 `node --check` 通过；定向 `git diff --check` 通过。
- 新默认 Release 的嵌入补丁与四行 UI 尚待统一构建、Manager 修复后执行
  `node scripts/verify-native-subtasks-live.mjs`。旧 Release 原生控件证据与
  fixture 测试不替代新 Release 的端到端验收。
