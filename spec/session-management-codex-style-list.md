# 会话管理 Codex 风格会话列表

## 背景

当前会话管理页的 Codex 会话管理区域以状态行和普通卡片列表展示本地会话。用户希望该区域更接近 Codex 自身的项目/会话列表结构：按项目分组，项目名称作为分组标题，下面展示会话标题和相对更新时间。

## 目标

本次要完成：

- 将会话管理页中的 Codex 会话展示改为类似 Codex 的项目分组列表。
- 每个项目显示项目名称，项目下显示会话条目和相对时间。
- 保留现有刷新、删除、备份和本地数据库读取逻辑。
- 保留会话管理页中的历史会话修复模块；移除 Claude 会话诊断卡片；在 Codex 会话管理下新增同结构的 Claude 会话管理。

本次不包含：

- 修改 Codex 本地数据库读取逻辑。
- 修改删除会话的后端行为。
- 新增会话打开、编辑、归档或批量操作。
- 改动 Codex/Claude 注入脚本。

## 用户视角描述

用户进入“会话管理”后，Codex 会话区域应像 Codex 侧边栏一样，先看到“项目”，每个项目下面列出最近会话。会话条目显示标题，右侧显示“19 小时”“4 天”“1 周”等相对时间，鼠标悬停时显示删除入口。

## 最新布局要求

- 会话管理页不再展示 `Claude 会话诊断` 卡片。
- 第一行使用红框式紧凑排布：左侧 `历史会话修复`，右侧 `Codex 会话管理`。
- `Codex 会话管理` 在卡片内部滚动，避免撑高第一行导致历史修复下方出现大面积空白。
- `Claude 会话管理` 放在下一整行，继续复用同一套会话列表结构。
- `Claude 会话管理` 复用 Codex 风格的项目分组视觉结构，但使用独立 Claude 数据状态、刷新命令和带备份删除命令；细节见 `spec/claude-session-management.md`。

## 功能要求

- 使用 `cwd` 推导项目名称，优先取路径最后一级目录。
- 没有 `cwd` 时回退到 `rolloutPath`、`dbPath` 或“未归类项目”。
- 会话按项目分组，同一项目下按 `updatedAtMs` 从新到旧排序。
- 每组最多展示合理数量的会话，整体保留当前本地会话数量统计。
- 每个会话仍可通过原有删除动作删除。
- 空态仍显示没有读取到 Codex 本地会话。

## UI / 交互要求

- Codex 会话区域使用浅色、紧凑、侧栏式列表视觉，贴近用户截图中的 Codex 项目/会话列表。
- 项目标题行包含项目图标和项目名称。
- 会话行使用圆角浅底，标题单行截断，右侧时间对齐。
- 删除按钮不占据主要视觉，作为悬停/聚焦时出现的图标按钮。
- 列表高度可滚动；历史会话修复与 Codex 会话管理在第一行左右对齐，Claude 会话管理在下一整行，整体占满会话管理页宽度且避免左侧大面积空白。

## 数据与接口要求

- Codex 继续使用 `list_local_sessions` / `delete_local_session`。
- Claude 使用独立 `list_claude_sessions` / `delete_claude_session`，不得复用 Codex 数据结果。

## 技术约束

- 优先修改 `apps/claude-codex-pro-manager/src/App.tsx` 和 `apps/claude-codex-pro-manager/src/styles.css`。
- 不新增 npm 依赖。
- 不修改 `assets/inject/claude-chinese-inject.js`。
- 不修改 Codex 后端数据库结构。

## 交付范围

- `apps/claude-codex-pro-manager/src/App.tsx`
- `apps/claude-codex-pro-manager/src/screens.tsx`
- `apps/claude-codex-pro-manager/src/styles.css`
- `apps/claude-codex-pro-manager/src-tauri/tests/windows_subsystem.rs`
- 本规格文档与对应验收标准
