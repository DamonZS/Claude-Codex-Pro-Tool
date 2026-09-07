# Multica 原始 UI 与 Codex Runtime 适配移植验收

对应规格：[multica-upstream-ui-runtime-port.md](../spec/multica-upstream-ui-runtime-port.md)。

## 发布前门禁

- [ ] source manifest 固定上游 revision、每个复制/派生文件的来源、版权头和修改说明。
- [ ] 完整 Multica LICENSE 与 NOTICE 被打包；派生 UI 中保留上游要求的产品名、Logo、
  版权和归属，或者仓库内存在可审计的书面商业许可及品牌豁免。
- [ ] 上述任一项缺失时，直接上游 UI 代码不得进入 release。

## 页面与导航

- [ ] 左侧只出现 `我的任务`、`自动化`、`智能体` 三项，顺序稳定；没有 CCP `项目`、
  `Skill` 入口，Codex 原生项目区域未被移动、覆盖或改写。
- [ ] 三项分别挂载上游移植的 MyIssuesPage、AutopilotsPage、AgentsPage；源码和
  DOM 测试能证明不是旧 `renderer-inject.js` 的手写 board/list/card 实现。
- [ ] 页面保留上游的结构、筛选、排序、虚拟列表、弹窗、空态、加载态、错误态、
  无权限态和键盘行为；容器使用 Codex 原生背景而不引入第二个产品壳。
- [ ] 重注入、刷新、深链接、原生导航返回和错误边界不造成重复入口、遮挡、冻结或
  Codex renderer 崩溃。

## API 与数据契约

- [ ] 每个移植页面实际调用的上游 API 方法都在 Adapter 矩阵中有类型化实现、schema
  校验、缓存失效和错误映射；未实现方法明确返回稳定的 `capability_unavailable`，不
  伪造成功或静默降级为假数据。
- [ ] Issue/Autopilot/Agent 的 CRUD、拖拽/状态流转、revision/CAS、403、409、重试和
  断线恢复具有定向单元或集成测试。
- [ ] renderer 不能透传 URL、Authorization、header、shell、路径、环境变量、权限或
  任意 runtime action；日志/DOM/URL 中没有 API key、token 或完整 prompt。

## Codex 原生执行

- [ ] 执行、继续、取消、自动化触发和智能体分配只通过当前 Codex 页面 Host 创建或
  操作原生 task/thread/subagent；没有 Multica server/daemon/CLI、Codex app-server、
  Claude 或第二模型执行进程。
- [ ] 相同幂等键重复三次只得到一个控制面 run 和一个 Codex 原生执行对象。
- [ ] Host 离线、能力缺失、部分提交和事件 gap 均有可恢复状态；已存在 thread 不被
  静默替换。
- [ ] Skill 清单来自 Codex 原生 inventory/capability，未知、未安装、未受信任或不
  兼容的 Skill 阻止派发；不自动安装、运行 hook 或扩大 MCP 权限。

## 最终证据

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
