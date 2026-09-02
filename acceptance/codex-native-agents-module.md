# Agents 模块原生智能体验收

对应规格：`spec/codex-native-agents-module.md`

## 通过标准

1. 本地 Agent 集合为空而 bootstrap 含 `codex_native_agents` 时，Agents 路由显示原生子智能体条目。
2. 原生条目只显示有界元数据，不显示编辑、删除、启停或派发按钮。
3. 本地 Agent 集合非空时不重复混入原生快照。

## 验证方式

- `node --check assets/inject/renderer-inject.js`
- `npm --prefix apps/claude-codex-pro-manager run check`
- `cargo test -p claude-codex-pro-core multica_workspace --lib -- --nocapture`
- 静态检查确认原生 fallback 使用只读渲染分支。

## 非目标

- 不实现原生子智能体编辑、删除、重派发或状态写入。
