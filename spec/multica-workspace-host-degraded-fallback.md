# Multica 工作区 Host 降级回退

## 背景

“我的任务”同时包含本地 Multica 控制面和 Codex 页面 Host 能力。Codex
渲染器重载、CDP 目标切换或 Host API 尚未注入时，原生执行暂时不可用，
但本地任务和只读原生 SQLite 投影仍然可用。

## 目标

- 启动器 bootstrap 在 Codex Host 调用失败时回退到本地工作区快照。
- 不伪造 Codex runtime；执行能力仍显示为不可用，Host 恢复后由 watchdog
  重新注入。
- 记录一次受控诊断事件，便于定位真实断连原因。

## 非目标

- 不创建第二个 Codex 进程或替代 runtime。
- 不修改 Codex SQLite、任务正文或用户凭据。

## 验收

- Host 调用返回错误时 bootstrap 仍返回本地任务快照。
- 现有 launcher 单元测试、格式检查和构建通过。
