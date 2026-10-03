# 规格文档：移除 Multica 侧边栏功能

## 背景

CCP 当前通过注入脚本在 Codex 侧边栏中添加了三个 Multica 协作功能入口：
- **M 我的任务**（my-issues）
- **A 自动化**（autopilots）
- **G 智能体**（agents）

这些功能属于 Multica 协作系统的一部分，与 CCP 的核心定位（Codex 增强、供应商切换、插件管理）不符。用户要求移除这些功能。

## 目标

- 从 Codex 侧边栏中移除 Multica 相关的三个入口按钮
- 清理相关的注入逻辑和事件处理
- 保留其他 CCP 功能（插件、MCP、主题等）不受影响

**不包含**：
- 不修改 Multica 的后端数据结构（可能仍被其他功能使用）
- 不删除数据库中的 Multica 相关表（保持向后兼容）
- 不修改 Multica 工作区的实现逻辑（只是不再从侧边栏入口）

## 用户视角

**修改前**：
1. 用户打开 Codex
2. 左侧侧边栏显示：项目列表 → **M 我的任务** → **A 自动化** → **G 智能体** → 插件按钮
3. 点击这些按钮会打开 Multica 工作区界面

**修改后**：
1. 用户打开 Codex
2. 左侧侧边栏显示：项目列表 → 插件按钮（Multica 相关按钮已移除）
3. 侧边栏更简洁，只保留核心 CCP 功能入口

## 功能要求

### 1. 移除侧边栏模块定义
**位置**: `assets/inject/renderer-inject.js:4537-4541`

删除 `multicaWorkspaceSidebarModules` 数组定义：
```javascript
const multicaWorkspaceSidebarModules = Object.freeze([
  { key: "my-issues", label: "我的任务", icon: "M" },
  { key: "autopilots", label: "自动化", icon: "A" },
  { key: "agents", label: "智能体", icon: "G" },
]);
```

### 2. 移除侧边栏入口渲染逻辑
**位置**: `assets/inject/renderer-inject.js:5322-5394`

删除或禁用 `multicaWorkspaceEnsureEntry` 函数中创建和管理侧边栏按钮的逻辑。

关键修改点：
- 第 5324 行：`allowedRoutes` 基于 `multicaWorkspaceSidebarModules` 构建
- 第 5338-5390 行：遍历模块创建侧边栏按钮的循环
- 第 5391 行：设置 `multicaWorkspaceState.entry` 为 "my-issues"

### 3. 清理相关调用
搜索 `multicaWorkspaceEnsureEntry` 的调用位置，确保移除后不会导致其他功能报错。

### 4. 保留其他功能
**必须保留**：
- 插件按钮（CCP 主入口）
- MCP 服务器管理
- 主题中心
- 供应商配置
- 记忆辅助

## UI / 交互要求

- 侧边栏布局自动紧凑，移除的三个按钮不留空白
- 插件按钮位置上移，紧随项目列表
- 已打开的 Multica 工作区（如果有）不受影响（用户可以手动关闭）
- 无需显示"功能已移除"的提示（静默移除）

## 数据与接口要求

- 不修改后端 API 或数据库表结构
- 不影响 `codex_automation_runs` 等数据表的读写（可能被其他功能使用）
- 不删除 Multica 相关的 Tauri 命令（保持向后兼容）

## 技术约束

- 只修改前端注入脚本 `assets/inject/renderer-inject.js`
- 不修改 Rust 后端代码（除非发现必须清理的废弃逻辑）
- 不修改 React 管理界面（CCP 控制台）
- 保持注入脚本的其他功能正常工作

## 交付范围

### 必需
1. 修改 `assets/inject/renderer-inject.js`，移除三个侧边栏模块
2. 构建 Release 版本到 `target/release/claude-codex-pro.exe`
3. 手动验证：启动 Codex，确认侧边栏只有插件按钮，无 M/A/G 按钮

### 可选
1. 如果发现废弃的后端逻辑（仅被这三个功能使用），可以一并清理
2. 更新相关文档（如果有提到这些功能）

## 验收标准参考

详见 `acceptance/remove-multica-sidebar-features.md`
