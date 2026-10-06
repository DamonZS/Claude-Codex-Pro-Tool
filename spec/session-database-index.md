# 会话数据库索引

## 背景

蒸馏工作台需要快速查询本地和 Claude Desktop 的会话记录，用于：
- 按项目、时间、来源筛选会话
- 快速定位特定消息和上下文
- 统计项目维度的会话数、消息数
- 支持后续的 AI 驱动内容筛选和智能推荐

当前实现直接扫描 JSONL 文件，随着会话数增长性能下降。需要引入 SQLite 索引提供毫秒级查询。

## 目标

Phase 1（本次）包含：

- 创建 SQLite 数据库 schema（sessions、messages、projects 表）
- 实现 JSONL 文件索引服务（扫描 + 增量更新）
- 提供 Tauri 查询命令（按项目/来源/时间筛选、分页）
- 基础前端组件展示索引功能

Phase 1 不包含：

- 后台自动索引服务（下一阶段）
- 全文搜索（FTS5）
- 消息内容的语义索引
- 与蒸馏工作台的完整集成（仅演示组件）

## 用户视角

开发者在管理工具中可以：
- 手动触发会话索引重建
- 查看索引后的会话列表（按项目分组）
- 快速筛选特定来源的会话（claude/codex）
- 查看基本统计信息（会话数、消息数）

## 功能要求

### 数据模型

1. **sessions 表**：会话元数据索引
   - session_id (主键，从文件名提取)
   - project, title, source ("claude" | "codex")
   - created_at, updated_at (毫秒时间戳)
   - message_count, file_path, file_mtime, indexed_at

2. **messages 表**：消息索引（轻量级，用于快速导航）
   - message_id (主键，从 JSONL 行中的 uuid 提取)
   - session_id (外键 → sessions)
   - role ("user" | "assistant")
   - created_at, content_preview (前 200 字符)
   - token_estimate, line_number (JSONL 行号)

3. **projects 表**：项目聚合统计
   - project_name (主键)
   - session_count, message_count
   - last_activity, source_mask (位掩码：1=claude, 2=codex, 3=both)

### 索引行为

- 扫描 Claude Desktop 和 Codex 的会话目录
- 根据 file_mtime 判断是否需要重新索引
- 批量插入，每 1000 条消息提交一次
- 支持增量更新和完全重建

### 查询接口

Tauri 命令：
- `query_sessions(filter, pagination)` - 查询会话列表
- `get_project_stats()` - 获取项目统计
- `start_session_indexing()` - 手动触发索引

过滤条件：
- project: 项目名称
- source: "claude" | "codex"
- date_range: { start, end } 毫秒时间戳

分页：
- page, page_size
- 返回 { items, total, page, total_pages }

## 数据与接口要求

- JSONL 文件保持为唯一真实来源（append-only，不可变）
- SQLite 仅作为查询索引，导出时仍从 JSONL 读取
- 数据库路径：`~/.claude-codex-pro/session-index.db`
- WAL 模式提供更好的并发读性能

## 技术约束

- 使用 `rusqlite` crate，已在依赖中
- 数据库操作使用事务保证一致性
- 大文件索引时使用 SAVEPOINT 批量提交
- 测试使用临时目录，不触碰用户真实数据

## 交付范围

- `crates/claude-codex-pro-core/src/session_index/` 模块
  - schema.rs - 数据库 schema 定义
  - indexer.rs - 索引服务实现
  - query.rs - 查询接口
  - tests.rs - 单元测试
- Tauri 命令集成（`apps/claude-codex-pro-manager/src-tauri/src/commands.rs`）
- TypeScript 类型定义和 API 封装
- 演示组件 `SessionIndexDemo.tsx`
- 本规格文档和对应验收文档
- 单元测试全部通过
