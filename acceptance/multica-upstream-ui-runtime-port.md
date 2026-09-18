# Multica 原始 UI 与 Codex Runtime 适配移植验收

对应规格：[multica-upstream-ui-runtime-port.md](../spec/multica-upstream-ui-runtime-port.md)。

## 当前证据状态（2026-09-19 归并）

下列勾选按对应层级的已记录证据判定：单元/集成/DOM 测试、真实原生执行、
Release 实机验证分别记录，不互相替代。最终 property-inclusive Release 已构建、
启动并重新注入；属性/视图偏好、详情返回和原生子任务实机验证已通过。仍保留的
上游能力边界见 [本轮交付记录](ccp-delivery-2026-09-19.md)。
文末 2026-09-18 各阶段计数、失败和产物均为历史检查点，不代表当前测试结果。

- 父线程统一前端验证：Manager `npm run check` 通过；`npm run vite:build`
  完整流水线退出 0，包含 **264 项 workflow 测试、25 项注入测试**、TypeScript、
  构建产物离线挂载测试及两套 Vite 构建。
- 固定前端产物后，`cargo test --workspace -j 1` **退出 0**：
  Core 单元 **704 passed / 7 ignored**，Manager 单元 **71 passed / 2 ignored**；
  全部集成、launcher/data 和文档测试通过。去除六次嵌套子进程重复计数后，
  汇总为 **1521 passed / 10 ignored**（原始含重复计数为 1527 passed）。
- `cargo fmt --check`、`git diff --check`、发布 LICENSE/NOTICE 字节核验通过。
  日志：`%TEMP%/ccp-final-property-frontend.log`、
  `%TEMP%/ccp-final-stable-workspace.log`。这部分引用父线程实际运行结果，
  本次文档归并未重新运行构建或测试。
- 中间默认 Release SHA-256：
  `6B4E39CEAE4CC5D0F007B8E13FD8DAF5FB3417794AFD4200BF15766F17528237`。
  Manager 使用内嵌资产修复注入后，三页 live 通过，1004×731、零可见告警。
  `node scripts/verify-native-subtasks-live.mjs` 退出 0：四个真实 ID/昵称可见，
  Descartes 精确原生子标签选中，父会话恢复，仅关闭本次新建 UI 子标签，
  `restored:true`。四个子线程 history 均无 turn 行，显示“状态待确认”而非推断完成。
  详见 [原生子任务验收](codex-native-subtasks-in-my-tasks.md)，证据目录：
  `evidence/native-subtasks-live/2026-09-18T16-40-56-170Z/`。
- 最终 property Release SHA-256 为
  `EDA0174FE2EB4FC8300FA6B3E851BFD7E2D99E09BFDE61FD4B160DD512D06F7D`，
  88,799,232 bytes；Manager 实际修复返回前端注入及后端在线均 true。
  三页、属性原始 UI、真实视图显隐/拖排重载、三类详情返回和四个原生子任务导航均
  通过，详见各自验收记录。

API 的实现及稳定错误边界以 [Adapter 矩阵](../apps/codex-workflow-surface/API_ADAPTER_MATRIX.md)
为准：附件、Squad 等已列明缺口不因测试全过而变成完整上游行为支持；原生运行时
设置权威和云服务非目标继续保留。未勾选的综合项表示尚未汇齐该项全部证据，
不自动推导新增功能或要求重复已有真实付费执行。

## 发布前门禁

- [x] source manifest 固定上游 revision、每个复制/派生文件的来源、版权头和修改说明。
- [x] 完整 Multica LICENSE 与 NOTICE 被打包；派生 UI 中保留上游要求的产品名、Logo、
  版权和归属，或者仓库内存在可审计的书面商业许可及品牌豁免。
- [x] 上述任一项缺失时，直接上游 UI 代码不得进入 release。

依据：固定 revision 的 source-manifest/派生哈希测试、发布工作流门禁及 LICENSE/NOTICE
字节核验已通过；归属在已验证 UI 中保留。此勾选不代替规格中适用商业分发场景的
独立许可要求，也不提前确认尚在构建的最终产物。

## 页面与导航

- [x] 左侧只出现 `我的任务`、`自动化`、`智能体` 三项，顺序稳定；没有 CCP `项目`、
  `Skill` 入口，Codex 原生项目区域未被移动、覆盖或改写。
- [x] 三项分别挂载上游移植的 MyIssuesPage、AutopilotsPage、AgentsPage；源码和
  DOM 测试能证明不是旧 `renderer-inject.js` 的手写 board/list/card 实现。
- [ ] 页面保留上游的结构、筛选、排序、虚拟列表、弹窗、空态、加载态、错误态、
  无权限态和键盘行为；容器使用 Codex 原生背景而不引入第二个产品壳。
- [ ] 重注入、刷新、深链接、原生导航返回和错误边界不造成重复入口、遮挡、冻结或
  Codex renderer 崩溃。

## API 与数据契约

- [ ] 每个移植页面实际调用的上游 API 方法都在 Adapter 矩阵中有类型化实现、schema
  校验、缓存失效和错误映射；未实现方法明确返回稳定的 `capability_unavailable`，不
  伪造成功或静默降级为假数据。
- [x] Issue/Autopilot/Agent 的 CRUD、拖拽/状态流转、revision/CAS、403、409、重试和
  断线恢复具有定向单元或集成测试。
- [ ] renderer 不能透传 URL、Authorization、header、shell、路径、环境变量、权限或
  任意 runtime action；日志/DOM/URL 中没有 API key、token 或完整 prompt。

## Codex 原生执行

- [ ] 执行、继续、取消、自动化触发和智能体分配只通过当前 Codex 页面 Host 创建或
  操作原生 task/thread/subagent；没有 Multica server/daemon/CLI、Codex app-server、
  Claude 或第二模型执行进程。
- [x] 相同幂等键重复三次只得到一个控制面 run 和一个 Codex 原生执行对象。
- [ ] Host 离线、能力缺失、部分提交和事件 gap 均有可恢复状态；已存在 thread 不被
  静默替换。
- [ ] Skill 清单来自 Codex 原生 inventory/capability，未知、未安装、未受信任或不
  兼容的 Skill 阻止派发；不自动安装、运行 hook 或扩大 MCP 权限。

## 最终证据

### 完整交付补齐门禁

- [x] 属性创建、编辑、归档、任务属性增删以及视图偏好经真实持久化重载验证；非法类型和 CAS 冲突有测试。
- [x] AI Builder 新建、继续、草稿保存、运行时选择和取消有原生执行绑定，重载后恢复相同会话。
- [x] Webhook 创建、轮换、验签、投递记录、重放及幂等派发均有测试和真实本机链路证据。
- [x] 原页面可达的协作者、订阅/反应、状态排序和快捷操作接通；API 矩阵逐项记录实现与验收证据。
- [x] 本地 Windows UI 通道验证三页及真实执行；浏览器工具认证错误不再被视为产品 UI 故障或代码实现停止条件。
- [x] 最新默认 Release、许可证包、截图、原生执行映射、进程和配置保留证据齐全后，才标记本轮交付；矩阵中的未实现上游能力仍按边界记录。

证据层级说明：CRUD/CAS/403/409/恢复及协作者、订阅/反应、排序的勾选依据
Adapter/Core/原组件定向测试与本轮全套通过；快捷操作另有下文真实 binding/thread
和重放证据，不宣称每个共享控件都完成逐项实机测试。Builder 勾选覆盖当前 Codex
runtime/default 选择、新建/继续/取消、保存与重载同线程，不表示额外模型覆盖生效。
Webhook 验签/非法凭证/轮换重试由测试覆盖；创建、一次性 token 轮换、三次 HTTP
投递去重、原生完成与显式重放由已记录本机链路覆盖。安全、恢复与 Skill 综合项
仍保留未勾选，待把相应断言及最终产物证据逐项关联，避免以全套计数替代证明。

必须提供实际通过的命令和产物：

```powershell
npm --prefix apps/codex-workflow-surface run check
npm --prefix apps/codex-workflow-surface run test
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo fmt --check
cargo test --workspace
cargo build --release
```

还必须使用最新 `D:\\Project\\Claude-Codex-Pro-Tool\\target\\release` 产物实际启动、
重新注入 Codex，并保存三页截图、原生 thread 映射、进程树、配置摘要和发行许可证
包检查结果。缺少任一真实运行证据不得宣称完成。

## Historical Integrated Verification: 2026-09-18

Historical status at this checkpoint: partial implementation. Current test and
intermediate Release status is recorded above; source/DOM and live evidence
remain separate. The failures and pending items below describe their dated stage.

### Passed Checks

- Manager `npm run check`: passed.
- Final Core library run: `cargo test -p claude-codex-pro-core --lib -j 1
  --quiet -- --test-threads=1`: 611 passed, 7 ignored, 0 failed (618 total).
  A preceding parallel test process became suspended and was terminated after
  verifying its exact project executable path; the serial rerun above completed.
- Manager `npm run vite:build`: passed, including 15 renderer integration tests,
  workflow TypeScript checking, 138 workflow tests, workflow production bundle,
  one built-IIFE offline/reinjection smoke test, and Manager Vite build.
- Workflow test breakdown: 117 adapter, 13 actual upstream page/DOM workflows,
  and 8 host/provenance tests. Original Issue create, Manual Agent create,
  Autopilot create with Agent picker/schedule, pause, navigation, theme and
  ShadowRoot portal ownership are covered with a local fixture bridge.
- Core integration: `cargo test -p claude-codex-pro-core --test cdp_bridge
  --test multica_workspace_fail_open --test workflow_surface --test bridge_routes
  -j 1 --quiet`: 96 + 3 + 2 + 37 passed.
- Manager release source contract: `cargo test -p claude-codex-pro-manager
  --test windows_subsystem github_auto_release_workflow_builds_installers_with_v0_tags
  -j 1 --quiet`: 1 passed.
- Release workflow verifier and 5 notice staging tests passed. Complete LICENSE
  and NOTICE were staged byte-for-byte under
  `target/release/resources/third-party/multica/`.
- Workflow `npm ci --ignore-scripts --dry-run`: passed; CI uses its lockfile.
- `git diff --check`: passed. Original unrelated changes remain in the worktree.
- Final combined Node injection/artifact/notice run: 21 passed. Parser regression
  additionally verifies explicit-null drag clearing and custom-status mutation
  access through the bridge whitelist.

### Implemented Contracts

- The pinned upstream closure is repository-local with per-file hashes and
  retained attribution. Three original pages and create/detail dependencies
  mount inside the existing injection host; the handwritten fallback is not used.
- Core owns manual/scheduled occurrence identity, UTC/timezone-aware cron, pause,
  restart cursors and bounded catch-up. Only the current Codex page Host dispatches.
- Agent access (`private/public_to`) is separate from native execution approval.
  Quick-create and drag assignment reserve a real binding; native IDs, errors and
  actual run state flow back through the adapter.
- Workspace mutation receipts survive adapter reconstruction when the original
  key is retained. Core hashes actual operation payloads and rejects mismatches.
  Multi-step writes use distinct stable keys; pending assignment recovery is tested.
- Agent concurrency is checked under the execution-store lock for dispatch,
  explicit create and continuation. Unresolved leased work remains capacity-bound;
  distinct concurrent continue commands for one binding are rejected. Polling the
  same previous turn can be reconciled when committing a native continuation.
- Nonempty model/thinking/service-tier overrides stop execution with a stable
  unsupported-setting error; the native selector remains authoritative.

### Historical Completion Checkpoints

### Completion Pass Live Evidence: 2026-09-18

- The old Pangu UI was traced to launching `H:/Claude Codex Pro/claude-codex-pro.exe`.
  The project Release at `D:/Project/Claude-Codex-Pro-Tool/target/release/claude-codex-pro.exe`
  was then launched explicitly; process path and native Manager UI confirmed the
  memory entry was absent. User memory databases were retained.
- Two independent native failures were reproduced and repaired: the audited
  AppServerManager export is `Jpn`, not `Qfe`; Zod JIT compiled with `new Function`
  during native callbacks and violated the existing Codex CSP. The entry now
  configures Zod `jitless` before schema imports; no CSP or login change was made.
- `node scripts/verify-workflow-live.mjs --screenshots` passed all three original
  routes in the current Codex page. Captures are under `acceptance/evidence/multica-live/`.
  Native read-only probe returned Codex plus thread/start, thread/read, turn/start,
  turn/interrupt and skills/list. These checks do not start another runtime.
- The original manual Agent form created `CCP 验收 20260918`. Its assignment form
  created issue `c34fbc68-85aa-4705-a7e0-233d83b2cb55`; background dispatch produced
  binding `binding:68ac79f2234e7b38`, native thread
  `01a0b42e-e1a6-7bf3-b325-59eda21f5c98`, turn
  `01a0b42e-e25f-7723-9e3e-7ec403aeeb23`. Native readback confirmed completed,
  an agentMessage containing `CCP_NATIVE_OK`, and binding completed. The fixture
  requested no tools or file changes and remains visible for inspection.
- The same test exposed the upstream Multica CLI gate and completed-turn Continue
  rejection. Both are tracked in this completion pass, not counted as passed yet.
- Source fixes are being integrated into the default Release. Hot-loaded live
  evidence above is separate from final packaged-binary verification.

### Subsequent Completion Evidence: 2026-09-18 Evening

- Integrated frontend pipeline: 199 workflow tests, 17 injection tests,
  TypeScript checks, built-artifact CSP/mount test and both production builds
  passed. Core integrations: bridge 45, CDP 96, fail-open 3, workflow 2 passed.
  Multica Core unit subset: 157 passed. Full Core library earlier in this pass:
  664 passed, 7 ignored. These are separate runs, not a full workspace pass.
- Native Continue on binding `binding:68ac79f2234e7b38` reused thread
  `01a0b42e-e1a6-7bf3-b325-59eda21f5c98`. Three identical requests returned the
  same turn `01a0b470-9de8-7090-a16f-cf82d3bee493`; changed content under the same
  key returned `execution_command_idempotency_conflict`. This exposed and fixed
  revision-before-receipt checking; signed result snapshots now survive reload.
- Original AI Builder UI created session `81997a0d-9174-4b98-b5b0-3d6513d2e866`.
  It generated and persisted a draft, then updated its name on continuation.
  Native thread `01a0b456-87b7-7d32-9772-b07509d52a32` contains completed turns
  `01a0b456-88ad-7f11-982f-370d6f0fd411` and
  `01a0b457-c1b7-7bf0-bf30-6b7ffd8a0671`; cancelling the third turn
  `01a0b458-8f4c-7561-8e53-eb9440eb95ed` was verified as `interrupted`.
  The visible form received the real generated draft; screenshot `builder-draft.png`.
- Local bridge property fixture `ccp-live-property-20260918`: create/edit,
  numeric issue value, invalid string rejection, value removal and archive all
  passed with persisted reload. Preference fixture `ccp-live-preferences-20260918`
  persisted. These are native bridge tests, not full property-control UI proof.
- Original Webhook form created automation `4d5353d3-e415-4252-ad53-3cb9e2a7bac4`
  and trigger `7cb32add-47d2-47a4-b3f7-0beb2d9db98c`. Its old UI exposed an
  unusable app-origin address; actual helper-port URLs and separate transient
  credentials have since been implemented. Live ingress verification follows.
- Normal launcher had omitted new trait delegations while Manager repair used
  Core directly. All Multica methods now delegate, share the same page transport,
  and use the selected helper port; launcher 17 tests passed. Final executable
  must include this last change; previous intermediate hashes are historical.
- Latest full Core library rerun: 677 passed, 7 ignored (684 total).
- Original Webhook detail displayed the selected loopback helper URL. UI token
  rotation returned a masked one-time credential with show/hide/copy/dismiss;
  the token was used only in memory, then dismissed without a secret screenshot.
  Three identical HTTP POSTs returned 202 and the same delivery
  `delivery-10aaf6b09f9c6410626dc2208149a1c245ea02d9e7c515a6f0db82e92017ad10`,
  with duplicate false/true/true. One binding `binding:1e3462d4c96aac7c` produced
  native thread `01a0b477-77de-7fb3-996b-398cfd00ffe8`, turn
  `01a0b477-78c2-7a61-81c3-8c0197bfe94b`; native readback was completed and
  contained `CCP_WEBHOOK_OK`. UI history showed completion and three receives.
- Original delivery-detail Replay created a distinct delivery linked to the
  original and native thread `01a0b479-1caa-70b3-bb34-4f5afa967d30`.
- Builder resume after Release reinjection restored the original session,
  native thread, draft name and four persisted messages. Create-from-draft then
  exposed a missing conversation-starters capability declaration; tracked in
  the final follow-up, not counted as successful creation here.
- Live quick-action run produced comment `476e585d-6c70-5e72-ac65-305e0512091b`,
  binding `binding:52e195bb3abc1026`, thread
  `01a0b47c-270b-7633-a00d-59a5f78a3442`, turn
  `01a0b47c-27b6-71f0-abe2-ecc38c00649d`. Identical replay returned the same
  comment and `queued` outcome, with no second execution.

### Historical Limitations Before This Completion Pass

The auth/tool-access, full-workspace and formatting failures below are historical,
superseded by the working Windows UI channel and the 2026-09-19 exit-0 workspace,
fmt and diff results above. The former endpoint gap list is also historical;
consult the current API matrix for remaining boundaries. Keep these records for
traceability, not as current blockers or current test failures.

- UI connection returns `unsupported Codex auth method: apikey`, with empty
  browser/app inventories. Live screenshots, native thread/Skill execution,
  reinjection, process-tree and unchanged-configuration evidence remain open.
- See `apps/codex-workflow-surface/API_ADAPTER_MATRIX.md` for exact missing
  endpoints: AI Agent Builder, Agent environment, collaborator grants, property
  catalog and per-property operations, view preferences, Webhook provisioning/
  token rotation/deliveries, quotas, status reorder and some shared detail actions.
  These return explicit errors, not fabricated successful results.
- Full `cargo test --workspace --no-fail-fast -j 1` did not pass: two Manager
  configuration-normalization tests, three Claude patch source contracts and four
  Manager doctests failed outside this task's changed workflow behavior. The Core
  `detached_helper_rejects_unverified_port_conflict` integration test stalled over
  five minutes; only its verified project test process was terminated. Subsequent
  targeted fixes/reruns are listed above, not presented as a full-suite pass.
- Parallel full Cargo linking first hit MSVC LNK1102; serial builds were used.
- Workspace formatting check reports existing differences in Manager
  commands/lib/windows_subsystem, Core protocol_proxy/bridge_routes, and data
  storage_adapter; workflow-owned Rust sources are formatted. No blanket
  formatting rewrite was applied to those unrelated files.
- Builds warn about CSS `::highlight` optimizer recognition and large bundles.
  The workflow JS is about 18.7 MB and CSS about 1.7 MB before transport compression;
  real-window injection performance is not yet measured.

### Historical Intermediate Artifacts

- Final `cargo build --release -j 1`: passed (5m 55s). Latest default executable:
  `D:\Project\Claude-Codex-Pro-Tool\target\release\claude-codex-pro.exe`,
  86,052,864 bytes, modified 2026-09-18 17:49:26 Asia/Shanghai.
  SHA-256: `6b1a403b19d4961c70bd853fb0117beb36ef0cc7c6f4fd31697b4040e86a34d5`.
  This artifact includes the final parser and concurrency changes. It has not
  been used to restart/reinject Codex; the blocked live UI checks remain open.
- Manager frontend preview: `http://127.0.0.1:1420/`, HTTP 200 verified. It is not
  a substitute for the desktop Host or injected workflow execution.
- Previous executable retained at
  `D:\Project\Claude-Codex-Pro-Tool\target\release\claude-codex-pro.before-multica-20260918-172335.exe`.
  Only the two processes whose executable path exactly matched the project's
  old default Release were stopped for replacement. Codex was not restarted.
- Final bundle hashes (SHA-256):
  JS `817680958b758c031bc64174e27c89dd0791fad499e6226f3fdc0e5cd4a03f45`;
  CSS `8ae4ad6013b806673c8d22f1a127a3cecfba7d5bded8a7d7e1e6dbc73802314f`.
- Measured generated directories: `target/debug` 71.25 GiB, `target/release`
  4.34 GiB, workflow dist about 0.02 GiB. No broad cache deletion was performed.
  The generated `apps/codex-workflow-surface/node_modules-before-port/` backup
  remains untracked after its cleanup attempt was blocked; it is not a dependency
  or distribution input and must not be included in a later source submission.
- No database files are present in the Git diff. Original Pangu removal, session
  deletion, telemetry and titlebar edits were retained. No commit or push was made.

### Screenshot Regression Follow-up: 2026-09-18

- The screenshot's conversation-starters error came from an uninitialized upstream
  config store, not a missing persistence field. The adapter supplies `/api/config`;
  mount waits for it before making the original forms ready and resets on disposal.
- On the current Codex page, the original saved Builder session was reopened and
  its actual Create-and-open button succeeded. Agent
  `f3e8ddc7-2a4b-4500-87c2-731c7bb64365` was read back from Core with both conversation
  starters intact. Capture: `evidence/multica-live/builder-created.png`.
  This initial proof used the freshly built frontend hotloaded into the existing
  Release; embedded-binary reinjection verification is recorded separately below.
- Run-only reservations previously counted all unrelated null-issue bindings as
  one retry chain. The fix resets independent occurrences and retains explicit
  parent retries. Store and manual-route regressions cover four occurrences and
  replay; the focused execution-store suite passed 23 tests.
- Fresh frontend pipeline: 202 workflow tests, injection tests, TypeScript, built
  CSP/mount test and both Vite builds passed. Manager `npm run check` passed.
- Fresh `cargo test -p claude-codex-pro-core --lib -j 1`: 680 passed, 7 ignored.
  This includes the dispatch-counter and independent manual-occurrence fixes.
- Fresh launcher tests: 8 unit and 9 source-contract tests passed. Bridge 45,
  CDP 96, fail-open 3 and workflow artifact integration 2 passed. Timeline,
  titlebar and notice-stage Node suites passed; release workflow and installed
  LICENSE/NOTICE byte-for-byte checks passed. `git diff --check` passed.
- With the rebuilt Release, the original automation Immediate-run button created
  `autopilot-run-1789736233995-3`, binding `binding:bf3c28b51d78ba61`, native thread
  `01a0b497-f012-7ce3-9453-64202809469c`, turn
  `01a0b497-f0a0-79f3-a484-ad3cb6ad30f2`. Both Core and native readback were
  completed, with `CCP_WEBHOOK_OK` present and `attemptNo:1`. The previously pending
  manual occurrence also recovered without a duplicate occurrence. Capture:
  `evidence/multica-live/manual-run-completed.png`.
- `verify-workflow-live.mjs --screenshots` without hotload passed all three routes
  after Release frontend repair. Native Manager reported zero Pangu labels,
  one original Token usage row and no alert.
- Launcher formatting differences introduced by this workflow were fixed.
  `cargo fmt --check` still reports pre-existing differences in Manager
  commands/lib/windows_subsystem, Core protocol_proxy and data storage_adapter.

### Historical Verified Default Release: 2026-09-18

Property UI source follow-up (2026-09-19): the pinned PropertiesTab closure now
has a secondary My Issues route and return control. Local management comes from
Core bootstrap permissions, preserving member identity. Original catalog/value
controls and actual builtin:/view: preference IDs are exercised by DOM tests;
the focused six-file suite passed 191 tests and TypeScript passed. The final
property-inclusive bundle/Release and live persistence check remain separate;
see [property UI acceptance](multica-property-management-ui.md). Historical
Release evidence below predates this source follow-up.

- Final `cargo build --release -j 1` passed. Default executable:
  `D:/Project/Claude-Codex-Pro-Tool/target/release/claude-codex-pro.exe`,
  88,485,376 bytes, modified 2026-09-18 20:59:16 Asia/Shanghai.
  SHA-256: `10507712ca7615d375879c03bba85d0c87216740c0ffafcb57f0d9dbdf24e255`.
- Exact project executable launched as PID 42544. Frontend repair used its
  embedded assets. The three-route live verification passed again without
  `--reload`. The saved Builder Agent and both starter labels persisted;
  capture `evidence/multica-live/builder-created-final-release.png`.
- Original Webhook Replay created
  `delivery-696cbe272b13455aaf60486809c37ee9512aa44a0e2de293136ed5c0f790e214`,
  with `dispatch_attempts:1`. Run `autopilot-run-1789736316768-4` completed in
  native thread `01a0b49a-8a92-7c02-87d8-93bb9c9568c5`, turn
  `01a0b49a-8b20-70c1-976d-f507690fe4f3`; native readback contained the expected
  reply. Pre-fix historical counters were not rewritten.
- No user database, Codex authentication or installed H-drive executable was
  changed in this follow-up. The existing installation remains an older build;
  use the project Release above for this verification.
- This closes the reported Builder creation regression and the independent
  manual-run/dispatch-counter regressions. It does not mark the full-port gates
  complete at that checkpoint: attachments, squad execution and native-authoritative
  runtime setting boundaries remain documented in the API matrix. Handoff/run
  controls and the full-workspace result have since advanced; use the current
  matrix and the 2026-09-19 summary above. Final property UI/return live evidence
  and the property-inclusive Release remain separate pending gates.
