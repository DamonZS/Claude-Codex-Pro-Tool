# 管理器页面导航性能修复

## 背景

管理工具（`apps/claude-codex-pro-manager`）在点击左侧导航切换到「工具与插件」「会话管理」「维护」「设置」「关于」等页面时，界面明显卡顿，进程 CPU 占用急速上升，导致风扇狂转。这是可复现的真实性能缺陷，不是单次首屏加载慢。

经静态分析定位到两条相互叠加的根因（均在前端，`src-tauri` Rust 后端命令本身已在既往提交中改为 async + spawn_blocking，不是本次目标）：

- 根因 A（点火 / 主因）：`src/App.tsx` 中 `useEffect(() => { void refreshRoute(route); }, [route])`（约 1383 行）在每次切页时触发 `refreshRoute`。而重页面一次性并发触发大量重活后端命令。以「工具与插件」为例（`refreshRoute` 的 `target === "tools"` 分支，约 1335–1357 行），单次切页并发调用约 10 个后端命令（`refreshPluginHub` / `refreshCodexPluginMarketplace` / `refreshClaudeDesktopOrgPlugin` / `refreshClaudeDesktopMarketplace` / `refreshClaudeDesktopDevMode` / `refreshClaudeContextEntries` / `refreshOverview` / `refreshClaude` / `refreshWatcher` 等），随后还串行执行 `refreshContextEntries` 与 `promptAndRepairPluginRepositories`。这些命令包含读取 MSIX 包、扫描插件仓库、跑 git 等重活。「会话管理」（5 路并发）、「维护」（4 路并发）同理。切一次页即触发多路重活并发，形成 CPU 尖峰；连续点几个页面即形成持续高负载与风扇狂转。

- 根因 B（放大器）：`src/screens.tsx` 中的页面级组件（`ToolsAndPluginsScreen` / `SettingsScreen` / `SessionManagementScreen` / `MaintenanceScreen` / `AboutScreen` 等）未做 `memo` 化，且 `src/App.tsx` 约 1421 行的 `actions` 对象在每次 render 都新建一个新引用。加载期间 `busyCount`（约 233 行，`run()` 每进出一次重活即 `setBusyCount` 一次）与 `notice` 频繁 setState，每次 setState 都把当前整屏组件树（含大列表、两个 `ContextManagerPanel`）全量重渲染。在根因 A 的多路重活期间，`busyCount` 会被反复加减（每个命令一进一出），触发密集的全量重渲染，进一步放大 CPU 抖动。

两者合力即「一点这些页就卡 + 风扇狂转」。

## 目标

### 包含

1. 消除切页时的「后端命令风暴」：区分「首屏必需」与「可延后 / 非阻塞」的数据加载，避免一次切页瞬间并发触发全部重活命令；对确属重活且非首屏必需的命令延后到首帧绘制之后（复用现有 `afterFirstPaint`）或按需触发。
2. 稳定 `actions` 引用，并对页面级 Screen 组件做 `memo` 化，切断「父组件任一 setState → 整屏全量重渲染」的放大链路。
3. 收敛加载期间由 `busyCount` 驱动的密集重渲染对整屏的冲击（例如让忙碌态不再导致整屏 Screen 重渲染，或将其影响限制在展示忙碌态的局部组件）。

### 不包含

- 不改任何用户可见文案 / 提示 / 标签的语言（不做汉化，也不动正在进行的汉化会话所改的字符串）。本任务只做性能结构调整，遵守 AGENTS.md 安全边界中「不得擅自对不属于当前会话任务范围的内容做汉化」。
- 不改 `src-tauri` 下的 Rust 后端命令实现（后端 spawn_blocking 已在既往提交完成，不在本次范围）。
- 不改导航结构、路由集合、页面视觉与交互布局。
- 不引入新的前端状态管理库或新依赖。
- 不改动数据来源、命令名、命令返回结构。

## 用户视角

用户在管理器左侧导航点击「工具与插件」「会话管理」「维护」「设置」「关于」之间切换时：

- 页面应立即切换并渲染出骨架 / 已有数据，不出现可感知的整屏卡死。
- CPU 占用只在必要的数据刷新期间短暂上升，且幅度显著低于修复前；连续切换多个页面不应造成持续高负载与风扇长时间狂转。
- 各页面的数据（插件仓库状态、上下文条目、会话列表、设置、概览等）最终仍能正确加载显示，功能与修复前一致。

## 功能要求

1. `refreshRoute`（`src/App.tsx`）各 `target` 分支的加载策略调整：
   - 保留每个页面「首屏渲染所必需」的最小命令集为进入即加载。
   - 将非首屏必需、或已知重活的命令（尤其是「工具与插件」分支里的插件仓库扫描 / 修复类：`refreshCodexPluginMarketplace`、`refreshClaudeDesktopMarketplace`、`promptAndRepairPluginRepositories`，以及 `refreshOverview` / `refreshClaude` / `refreshWatcher` 这类跨页可复用状态）改为延后到首帧之后触发或按需触发，避免与首屏加载挤在同一并发批次。
   - 必须保留既有的 `routeLoadEpochRef` 陈旧加载保护语义：延后执行的加载在触发前需检查是否已被更晚的切页取代（`isStaleRouteLoad`），避免陈旧写入。
   - 不得删除任何一项数据的加载（最终仍要加载到），只调整触发时机 / 批次。
2. `actions` 引用稳定化：用 `useMemo`（配合对其中回调按需 `useCallback`）或等效手段，使 `actions` 在依赖未变时保持同一引用；确保其内部各方法在其依赖未变时也稳定，以便下游 `memo` 生效。不得改变任何 action 的行为与签名。
3. 页面级 Screen 组件 `memo` 化：对 `ToolsAndPluginsScreen`、`SessionManagementScreen`、`MaintenanceScreen`、`SettingsScreen`、`AboutScreen`（及其接收 `actions` 的重型子面板，如 `ContextManagerPanel`）视需要用 `React.memo` 包裹，使其在 props 引用未变时跳过重渲染。
4. 忙碌态重渲染收敛：确保 `busyCount` / `notice` 的频繁变化不再触发当前整屏 Screen 的全量重渲染（例如通过上面 memo + 稳定 props 使 Screen 不因顶层忙碌态而重渲染，忙碌态只影响命令条 / toast 等局部）。
5. 保持功能等价：所有页面的数据仍正确加载与展示；已有的自动修复（`promptAndRepairPluginRepositories`、Codex 插件仓库自动注册 effect）逻辑与用户可见提示行为不变，仅触发时机可调整。

## UI / 交互要求

- 页面切换即时响应，先渲染已有 / 空态，再异步补数据；不得出现整屏白屏卡死。
- 加载中的忙碌指示（命令条 disabled、toast「running」态）行为保持不变。
- 不改任何文案、按钮、布局、配色、响应式行为。
- 空态 / 错误态 / 加载态的既有表现保持一致。

## 数据与接口要求

- 不新增、不删除、不重命名任何 Tauri 命令。
- 不改命令入参与返回结构。
- 各页面所需数据的最终来源与内容不变，仅调整前端触发时机 / 渲染策略。
- 保持既有错误处理：命令失败仍走 `run()` 的失败通知路径（silent 模式下不弹通知的语义不变）。

## 技术约束

- 技术栈：React + TypeScript + Vite（前端），Tauri（外壳）。仅改 `apps/claude-codex-pro-manager/src` 下的前端源码。
- 禁止改动区域：`src-tauri`（Rust 后端）、`crates/`、导航路由集合、任何用户可见文案。
- 遵守 React Hooks 规则：`memo` 化与 `useMemo`/`useCallback` 不得破坏既有 Hooks 调用顺序（参见记忆 [[frontend-memoization-done]] 中「组件内条件早 return 会违反 Hooks 规则」的约束）。
- 遵守 [[windows-subsystem-test-contract]]：`tests/windows_subsystem.rs` 按前端源码文本做存在性 / 切片断言，移动或改写前端代码可能撞测试；若因结构调整导致断言失配，需同步更新该测试的锚点，但不得借机改动被断言的文案文本本身。
- 不得回滚或干扰正在进行的汉化会话对 `src/` 字符串的改动（AGENTS.md 安全边界：不得回滚无关工作区改动）。
- 最小必要改动，避免无关重构。

## 交付范围

- 代码：`apps/claude-codex-pro-manager/src/App.tsx`、`apps/claude-codex-pro-manager/src/screens.tsx`（及必要时 `src/lib/` 内相关 helper）。
- 测试 / 验证：`npm --prefix apps/claude-codex-pro-manager run check`、`npm --prefix apps/claude-codex-pro-manager run vite:build` 通过；若触及 `windows_subsystem` 断言锚点，`cargo test -p claude-codex-pro-manager --test windows_subsystem` 通过。
- 文档：本 spec 与对应 `acceptance/manager-navigation-performance-fix.md`。
- 不交付：后端改动、文案改动、新依赖。
