# 盘古记忆 Runtime 状态同步修复

## 背景

用户截图显示 Codex 窗口标题栏已经出现“盘古记忆 0 codex:path:...”，说明注入脚本已运行并上报 runtime；但管理工具概览仍显示“等待 Codex memory runtime injection / 等待 Codex 注入”。本地日志也能看到新鲜的 `renderer.memory_runtime`，因此问题在 manager 状态读取和显示归类，不是简单的未注入。

## 目标

本次要完成：

- manager 后端能稳定读取最近的 `renderer.memory_runtime` 日志心跳。
- runtime snapshot 字段缺失或新增时不应导致整条 runtime 被丢弃。
- `idle` runtime 状态表示“已注入，等待真实对话消息”，不应显示成等待注入或异常。
- 不删除、不重置用户 `memory_assist.sqlite`。

本次不包含：

- 不自动确认待记忆候选。
- 不修改 Claude 中文注入脚本。
- 不强制关闭 Codex 做验证。

## 功能要求

- `MemoryAssistRuntimeSnapshot` 反序列化字段应有默认值。
- `latest_renderer_runtime_heartbeat` 应扫描足够多的近期日志，避免日志密集时 240 行窗口漏掉心跳。
- `enrich_memory_status` 遇到 `injected:true` 且 `status:"idle"` 时，必须设置 `codex_injected=true`，并给出“等待真实对话消息”的运行文案。
- 前端概览和运行状态行应把 `idle` 视为可用状态，而不是警告。

## 交付范围

- `apps/claude-codex-pro-manager/src-tauri/src/commands.rs`
- `apps/claude-codex-pro-manager/src/App.tsx`
- 定向回归测试

