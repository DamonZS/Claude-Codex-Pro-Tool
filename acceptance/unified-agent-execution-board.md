# 统一执行看板验收

对应 `spec/unified-agent-execution-board.md`。

- [x] 顶部独立列表移除；原生子任务与 Multica 任务位于同一个原始看板。
- [x] 排队、执行、完成、失败、中断、取消、未知正确映射，卡片标明真实状态。
- [x] 最新执行决定归列；普通未执行 Issue 及自定义状态保持，投影没有新增写入/派发。
- [x] issue/thread/binding 排重；无 Issue 的 Agent/自动化执行可见，闲置定义不假冒任务。
- [x] 筛选/分页/分组及工作中计数使用同一投影集合；超过 100 原生记录有界读取。
- [x] 自动刷新改变卡片列且不重复；隐藏停止额外轮询，错误保留旧卡片并标过期。
- [x] 虚拟卡片详情复用打开线程与父会话入口；字段编辑/删除受只读约束，原生拖动执行按补充要求开放，失败有可见反馈（组件验证，实机记录见末项）。
- [x] 原始 UI + adapter 行为回归、TypeScript、manifest、生产构建与必要 Rust 检查通过。
- [ ] 默认 Release 启动/重注入后真实卡片归列验证；记录产物/截图/数据保留证据。
- [ ] 原生卡片拖到进行中真实继续原会话；拖到待办真实排队；重复拖动不重复执行，失败回退，执行结束自动归类。
- [ ] 当前 Codex 版本实际点击打开子会话成功；恢复原父会话，父会话无新增智能体报告。

不以直接修改数据库状态获取实机通过；状态转换和失败边界用真实接口形状 fixture
驱动组件验证。用户明确要求的拖动执行经真实原生接口验证，导航另行验证；保留用户原数据库与会话。

## 2026-09-19 验证记录

- `npm --prefix apps/claude-codex-pro-manager run check`：通过。
- `npm --prefix apps/claude-codex-pro-manager run vite:build`：通过；内含 workflow
  类型检查、20 文件 / 312 个前端测试、33 个注入测试、1 个生产资源挂载测试。
  两个前端生产包构建成功；保留现有大 chunk、上游 CSS/Zod 注释警告。
- `cargo fmt --check`：通过。
- `cargo test -p claude-codex-pro-core --test workflow_surface --test cdp_bridge`：
  98 个测试通过；证明最新工作流 JS/CSS 参与注入指纹，并保留桥接入口契约。
- `node --test scripts/verify-unified-execution-board-live.test.mjs`：8 个通过。
  检查实机判据拒绝旧状态、错列、重复线程，以及必须显示的来源样本遗漏。
- 真实 MyIssuesPage 回归覆盖同卡片运行→完成→失败的刷新迁列、工作中 2→1、
  失败保留整批、重试恢复、5 秒刷新和隐藏停止；原始列表/表格虚拟行只读，
  普通行可选择；普通卡片移动时跳过虚拟邻居 ID。
- 原生 source 实机只读预检：近期 500 条、stale=false，含 inProgress/completed/
  failed/interrupted/unknown。本条不替代最终 Release 的页面验收。
- 最终后端定向回归：`cargo test -j 4 -p claude-codex-pro-core --test bridge_routes native_drag -- --nocapture`
  13 个通过；同包 `--lib codex_execution` 22 个、`--lib multica_execution_store`
  23 个、`--lib routes::tests` 28 个通过。明确未发送失败、持久幂等回执、排队转立即执行、
  超时后只读接管新 turn 均覆盖；测试仅使用临时 store 和 fake host。
- `cargo test -j 4 -p claude-codex-pro-launcher --test launcher_source_contract`：9 个通过，
  包含启动器对新增 native intent 接口的完整转发检查。
- 收尾再次执行 `cargo fmt --check`、`git diff --check`：通过。
- `cargo test --workspace` 曾中断，未计入通过结果；完整 bridge suite 曾出现一项
  测试预置响应过期，修正后相关 13 项定向测试通过，未宣称全套重新通过。

## 2026-09-19 用户测试构建

- `cargo build --release -j 4 -p claude-codex-pro-manager --bin claude-codex-pro`：
  通过，耗时 2m32s；已有未使用导入/函数警告保留。
- 产物：`D:\Project\Claude-Codex-Pro-Tool\target\release\claude-codex-pro.exe`。
  文件时间 `2026-09-19 21:12:06 +08:00`，88,613,888 bytes。
- SHA256：`39958855EA7FC266D609C8961916710767EFCA3BFFA6B6AD4F7226D3D2B5E408`。
- 为替换产物，仅停止该精确路径的旧 CCP 进程；Codex 保持运行。
  用户打开新版后点击“修复前端连接”，再测试拖动续跑/排队、终态归列与子会话导航。
  本轮构建后未启动应用、未重注入、未操作实机卡片；相应验收项仍待用户测试。

## 保留与边界

- 用户最新要求由用户自行实机测试。本次交付以代码收尾、已执行的自动检查和
  默认 Release 构建为准；上述三项实机检查保留未勾选，等待用户测试结果。
  本轮停止实机拖动、续跑和导航验收，不以之前的失败记录或局部探测代替通过。

- 查询投影不创建伪 Issue/Agent、不隐式派发模型任务、不直接修改原生数据库。
  原生拖动到进行中/待办是用户显式执行指令，使用同一原子线程和真实执行队列。
  原有 bootstrap 仍有初始化/恢复职责；不把它宣称为纯数据库读取。
- 原生最多读取近期 500 条；执行保持原有 5000 查询保护上限。闲置 Agent 定义
  仍在智能体页；已有执行才生成任务卡片。尊重已保存的范围、筛选、显示偏好。
- 对已有完整快照，来源故障保留整批并告警，包括故障期间已编辑但尚未重新
  查询确认的字段；恢复健康读取后显示新值。
- 并发查询共享同次加载；串行分组/分页仍会再次读取有界数据，不使用 TTL
  隐藏状态变化。大量历史任务的耗时以最终实机检查为准。
