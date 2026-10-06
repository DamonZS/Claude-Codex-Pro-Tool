# 会话索引功能演示

## 概览

会话索引功能提供了对本地会话数据的快速查询和筛选能力。

## 功能特性

### 后端 (Rust)

1. **数据库索引**
   - 使用 SQLite 存储会话元数据
   - 位置: `~/.claude-codex-pro/session-index.db`
   - 表结构包含: id, agent, project, session_path, title, started_at, updated_at, event_count, branch_name, commit_sha, pull_request_url

2. **Tauri 命令**
   - `build_session_index`: 扫描并索引本地会话
   - `query_sessions`: 分页查询会话，支持筛选
   - `get_session_detail`: 获取单个会话详情
   - `delete_session_index`: 删除索引记录

3. **自动索引**
   - 支持后台自动扫描和索引新会话
   - 增量更新，只索引变化的会话

### 前端 (React + TypeScript)

1. **路由集成**
   - 添加 `session-index` 路由
   - 位于侧边栏"会话索引"入口

2. **UI 组件**
   - `SessionIndexDemo`: 演示组件，展示基本查询和索引功能
   - 支持加载会话列表
   - 支持手动触发索引构建

3. **类型定义**
   - `SessionIndexRow`: 会话记录类型
   - `SessionIndexPagedResult`: 分页结果类型
   - `SessionIndexFilter`: 筛选条件类型
   - `SessionIndexBuildResult`: 构建结果类型

## 使用方式

1. 启动应用后，访问"会话索引"页面
2. 点击"重新索引"按钮扫描本地会话
3. 索引完成后自动加载会话列表
4. 每个会话卡片显示：
   - 标题
   - 项目和 Agent 信息
   - 事件数量
   - 分支和 PR 信息（如有）
   - 时间戳

## 后续扩展

- [ ] 添加高级筛选 (按项目、Agent、时间范围)
- [ ] 添加全文搜索
- [ ] 添加会话详情页
- [ ] 添加批量操作
- [ ] 添加导出功能
- [ ] 性能优化和缓存策略
