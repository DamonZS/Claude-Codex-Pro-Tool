# 验收标准：管理器页面导航性能修复

## 对应规格

本文件验证 `spec/manager-navigation-performance-fix.md`。

## 通过 / 失败标准

### 必须满足（全部为「通过」的硬性条件）

1. **切页命令批次收敛（对应根因 A）**
   - `src/App.tsx` 的 `refreshRoute` 中，「工具与插件」（`target === "tools"`）分支进入即执行的首屏并发命令数量，相比修复前的约 10 路**明显减少**；插件仓库扫描 / 修复类重活命令（至少包括 `refreshCodexPluginMarketplace`、`refreshClaudeDesktopMarketplace`、`promptAndRepairPluginRepositories`）改为延后到首帧之后（`afterFirstPaint`）或按需触发，不再与首屏加载挤在同一同步并发批次。
   - 「会话管理」「维护」分支同样区分首屏必需与可延后命令，不再一次性并发全部。
   - **失败判据**：任一目标页仍在切页瞬间无差别并发触发其全部原始命令集。

2. **不丢数据加载**
   - 每个页面修复前会加载的数据项，修复后最终仍会被加载（只是时机 / 批次调整）。逐分支比对 `refreshRoute` 修复前后，命令集合为「同一集合的重新编排」，不得有任何一项被永久删除。
   - **失败判据**：某页某项数据在任何路径下都不再加载。

3. **陈旧加载保护保留**
   - 延后 / 异步触发的加载在真正执行副作用前，仍通过 `routeLoadEpochRef` / `isStaleRouteLoad()`（或等效的 epoch 校验）判断是否已被更晚的切页取代；被取代时不写入 state。
   - **失败判据**：延后加载丢失 epoch 校验，可能把陈旧数据写进已切走的页面。

4. **`actions` 引用稳定 + Screen memo 化（对应根因 B）**
   - `src/App.tsx` 的 `actions` 对象在依赖未变时保持同一引用（`useMemo` 或等效）。
   - `ToolsAndPluginsScreen`、`SessionManagementScreen`、`MaintenanceScreen`、`SettingsScreen`、`AboutScreen` 中至少覆盖到卡顿页的页面级组件被 `React.memo` 包裹；接收 `actions` 的重型子面板（如 `ContextManagerPanel`）视需要同样 memo 化。
   - **失败判据**：`actions` 仍每次 render 新建，或目标 Screen 仍无 memo，使顶层 `busyCount`/`notice` setState 触发整屏全量重渲染。

5. **Hooks 规则不破坏**
   - 新增 / 调整的 `useMemo`/`useCallback`/`memo` 不改变既有 Hooks 调用顺序，无条件早 return 夹在 Hook 之间（见 [[frontend-memoization-done]]）。
   - **失败判据**：`npm run check` 报 Hooks 相关错误，或运行时 React 报「rendered fewer/more hooks」。

6. **行为与文案等价**
   - 不改任何用户可见文案 / 标签 / 提示（不做汉化，不动汉化会话所改字符串）。
   - 忙碌态（命令条 disabled、toast running）、空态、错误态、自动修复的用户可见行为不变。
   - 不改 Tauri 命令名 / 入参 / 返回结构。
   - **失败判据**：出现任何文案改动、命令签名改动，或忙碌 / 错误态可见行为变化。

7. **禁止改动区域未被触碰**
   - 未改 `src-tauri`、`crates/`、路由集合、视觉布局。
   - 未回滚正在进行的汉化会话对 `src/` 字符串的改动。

## 必需的验证方式

- **类型检查**：`npm --prefix apps/claude-codex-pro-manager run check` 通过（0 error）。
- **构建**：`npm --prefix apps/claude-codex-pro-manager run vite:build` 成功。
- **测试契约**：若结构调整触及 `tests/windows_subsystem.rs` 的断言锚点，`cargo test -p claude-codex-pro-manager --test windows_subsystem` 通过；未触及则说明未触及。
- **代码走查证据**：给出 `refreshRoute` 各目标分支修复前后的命令编排对比（哪些进入即加载、哪些延后 / 按需），证明「同集合重新编排、无丢项」。
- **memo 证据**：指出 `actions` 稳定化的实现点，以及被 memo 化的组件清单。

## 完成任务所需的证据

1. 修改文件清单 + 每个文件的改动摘要。
2. 上述三条命令的真实输出（check / vite:build /（如触及）windows_subsystem）。
3. `refreshRoute` 分支命令编排前后对比表。
4. 被 memo 化的组件与 `actions` 稳定化点说明。
5. 对照本文件逐条说明满足情况。

## 非目标 / 不在范围

- 不做后端 Rust 命令性能优化（已在既往提交完成 spawn_blocking）。
- 不做运行时 CPU 占用的数值化基准测试（本地无稳定基准环境）；性能改善通过「命令批次收敛 + 重渲染收敛」的结构性证据论证，而非跑分数字。
- 不改导航结构、视觉、文案、依赖。
- 不验证汉化会话本身的正确性（另属其任务）。
