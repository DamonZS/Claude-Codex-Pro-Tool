# 验收标准：移除 Multica 侧边栏功能

## 关联规格文档
`spec/remove-multica-sidebar-features.md`

## 验收项

### 1. 侧边栏按钮已移除
**验证方式**：手动检查

**通过标准**：
- 启动 Codex（通过 CCP 启动或直接启动）
- 左侧侧边栏**不显示**以下按钮：
  - **M 我的任务**
  - **A 自动化**
  - **G 智能体**
- 插件按钮（CCP 主入口）**仍然存在**且位置正常

**失败标准**：
- 仍显示任何一个 M/A/G 按钮
- 插件按钮消失或位置异常
- 侧边栏出现空白占位符

### 2. 其他 CCP 功能正常
**验证方式**：手动检查

**通过标准**：
- 点击插件按钮，CCP 主界面正常打开
- 主题切换功能正常
- 供应商配置正常加载
- MCP 服务器列表正常显示

**失败标准**：
- 任何 CCP 核心功能无法使用
- 控制台报 JavaScript 错误
- 界面卡死或无响应

### 3. 代码修改符合预期
**验证方式**：代码审查

**通过标准**：
- `assets/inject/renderer-inject.js` 中：
  - `multicaWorkspaceSidebarModules` 数组为空或已删除
  - `multicaWorkspaceEnsureEntry` 函数不再创建侧边栏按钮，或整个函数已禁用
  - 相关的事件监听器已清理
- 没有遗留的 `data-ccp-multica-nav="true"` 按钮创建逻辑

**失败标准**：
- 仍包含完整的三个模块定义
- 仍包含按钮创建和插入 DOM 的逻辑
- 注释掉代码而不是删除（应彻底删除）

### 4. 构建成功且文件可用
**验证方式**：命令行检查

**通过标准**：
```bash
# Vite 构建成功
cd apps/claude-codex-pro-manager
npm run vite:build
# 输出：✓ built in XXXXms

# Release 构建成功
cd ../..
cargo build --release -p claude-codex-pro-manager
# 输出：Finished `release` profile

# 可执行文件存在且大小合理（约 50-52MB）
ls -lh target/release/claude-codex-pro.exe
# 输出：-rwxr-xr-x 1 user group 51M ... claude-codex-pro.exe
```

**失败标准**：
- 编译错误
- 可执行文件不存在
- 文件大小异常（小于 40MB 或大于 60MB）

### 5. 无回归风险
**验证方式**：手动测试

**通过标准**：
- Codex 正常启动，无崩溃
- 项目列表正常显示
- 会话切换正常
- 模型菜单正常
- 插件菜单正常

**失败标准**：
- Codex 启动失败或崩溃
- 侧边栏完全不显示
- 其他注入功能失效

## 已知非目标

以下内容**不在本次验收范围**：

1. **后端数据清理**：
   - `codex_automation_runs` 表仍存在
   - Multica 相关的 Tauri 命令仍可调用
   - 不验证后端 API 的变化

2. **工作区清理**：
   - 如果用户之前打开过 Multica 工作区，可能仍有残留状态
   - 不要求清理已打开的工作区实例

3. **文档更新**：
   - 不强制要求更新 README 或其他文档
   - 如有提及这些功能的文档，可以后续清理

4. **测试覆盖**：
   - 没有自动化测试覆盖这些功能
   - 只进行手动验证

## 证据要求

交付时需提供：

1. **代码修改截图或 diff**：
   - 显示 `multicaWorkspaceSidebarModules` 已移除
   - 显示 `multicaWorkspaceEnsureEntry` 的关键修改

2. **Codex 侧边栏截图**：
   - 启动 Codex 后的侧边栏状态
   - 清楚显示只有插件按钮，无 M/A/G 按钮

3. **构建成功日志**：
   - Vite 构建成功的输出
   - Cargo Release 构建成功的输出
   - `target/release/claude-codex-pro.exe` 的文件信息

4. **功能正常的证明**：
   - 插件按钮点击后的界面截图
   - 控制台无 JavaScript 错误的截图

## 回归测试清单

| 功能点 | 预期行为 | 实际结果 |
|--------|----------|----------|
| Codex 启动 | 正常启动，无崩溃 | ✅/❌ |
| 侧边栏显示 | 只有插件按钮，无 M/A/G | ✅/❌ |
| 插件按钮 | 点击打开 CCP 界面 | ✅/❌ |
| 主题切换 | 可以切换主题 | ✅/❌ |
| 供应商配置 | 可以查看和编辑 | ✅/❌ |
| MCP 服务器 | 列表正常显示 | ✅/❌ |
| 控制台错误 | 无 JavaScript 错误 | ✅/❌ |

## 完成定义

满足以下所有条件才算完成：

1. ✅ 验收项 1-5 全部通过
2. ✅ 提供了所有必需的证据
3. ✅ 回归测试清单全部标记为 ✅
4. ✅ `target/release/claude-codex-pro.exe` 可用于用户测试
