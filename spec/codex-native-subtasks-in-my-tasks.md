# 我的任务中的 Codex 原生子任务

## 背景与目标

用户指出当前对话启动的 Descartes、Tesla 等原生子智能体没有出现在我的任务。
原始 Multica 页面仅消费 CCP Issue/执行绑定，丢失了旧版原生库存的可见入口。
补齐真实子任务可见性，保留原 Issue 页布局及所有已有任务功能。

## 工作流与交互

我的任务中增加紧凑的“Codex 原生子任务”区域；默认显示近期条目，可展开。
每项展示昵称（无昵称用短线程 ID）、父会话、最近更新时间和原生执行状态。
提供打开子会话和父会话操作，复用现有原生打开入口。错误显示并支持重试。
原生子会话可能仅存在于父会话的子智能体面板，不出现在主侧栏。打开行为优先
保留现有侧栏路径；缺少侧栏行时，核验 thread/read 返回 ID 后，读取已挂载
原生多智能体消息控件的 backgroundAgentOpener，仅在 canOpen(id) 为 true 时
调用 open(id)。不写 React 状态、不导入另一套 runtime、不创建线程或发送消息。
必须等待精确 ID 的原生 background-agent 标签被选中，或 subagents 面板加载完成
并选中该 ID，才隐藏工作区；单有父线程 active 或 opener 返回都不算成功。
原生入口缺失、目标不匹配或确认超时维持工作区并报告打开失败。
定时刷新有界元数据；切走页面停止该区域轮询；查询失败保留旧数据并标记过期。
空记录不假冒有工作，未知状态显示“状态待确认”。关闭/归档不是完成证据。

## 数据与接口

新增只读 workspace query resource `codex_native_agents`，沿用已鉴权查询接口及分页限制。
数据来自只读 Codex threads、thread_spawn_edges、thread_turns。
稳定键为原生子线程 ID，父子关联按真实边读取，昵称优先 agent_nickname。
最新 turn status 为执行状态来源，单独保留 edge open/closed；缺少状态用 unknown。
线程标题可能包含完整指令，不作为默认标题或日志输出；只返回有界必要元数据。
不得通过此读取创建 Issue、Agent 定义、执行绑定或新运行时，保留全部用户数据库。

## 交付与验证

Core 只读分页资源及 schema fixture 测试；前端独立区域及查询/打开/错误测试；
页面接入；构建；新默认 Release 的真实四个子线程可见证据。
