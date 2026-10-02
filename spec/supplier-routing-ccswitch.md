# 供应商与路由按 CC Switch 整改

## 背景
CCP 的供应商与路由模块问题较多：
- 前端和后端各生成一份配置。
- 切换供应商时不回填 live 配置。
- 保存和切换用的不是同一把锁。
- 回滚失败时错误被吞掉。
- `settings.json` 解析失败时会静默回退到默认值，下次保存就把用户配置覆盖掉。
- 故障转移和连通检测实际没有实现。
- 存在死代码和超大组件。

本次参考 CC Switch 3.20.4（MIT 协议）重做。

## 目标
1. 供应商存储改为 SQLite（`ccp.db`，位于 CCP 应用数据目录）。首次启动时从 `settings.json` 迁移：迁移前备份，保留原字段以便回滚，迁移失败时回退读 JSON。
2. 新增统一的供应商端点解析入口 `provider_endpoint::resolve`。代理、测试连接、蒸馏都改用它。
3. 切换供应商：
   - 所有应用共用一把串行锁。
   - live 文件只替换关键字段，并删除上一家独有的字段。
   - 每个文件第一次写入前做字节级备份。
   - 用 hash 比对后再原子 rename。
   - 失败时还原 live 文件和设置。
4. 路由：
   - 本地路由：开关、地址、端口、状态，端口冲突时自动换端口。
   - 自动故障转移：每个应用一个队列，熔断器三态。
   - 连通检测：超时 8s，仅超时重试 1 次，TTFB 超过 6s 标为"较慢"。
   - 整流器。
   - 全局出站代理。
   - Header/Body 覆盖对 Codex 也生效。
5. 设置文件解析失败时不得造成数据丢失：先保留损坏文件的副本，并记录错误。
6. 阻塞型命令改为 `spawn_blocking`。
7. 前端：把 `SupplierScreen` 拆分成多个组件，整页只用一个数据源；删除死代码和废弃命令。

## Agent 范围（用户确认：照 CC Switch 覆盖更多种 Agent，并加上 WorkBuddy、Cursor）
- **可路由接管**（走本地 HTTP 路由）：Claude Code、Claude Desktop、Codex、Gemini。
- **配置文件型**（直接写入 Agent 自身的配置，不经过路由）：Grok Build、OpenCode、OpenClaw、Hermes、Pi、MiniMax Code、WorkBuddy、Cursor。
- 每个 Agent 都可以单独覆盖配置目录，也可以在主页面显示或隐藏。
- 前端的单一来源是 `components/settings/contract.ts` 中的 `AGENT_APPS`。

## 非目标
- 云同步（WebDAV/S3）。
- OAuth 认证中心的第三方订阅登录。

## 约束
- 复用工作区已有的 `rusqlite 0.32 (bundled)`，不引入第二套 SQLite 驱动。
- 不读取任何系统提示词相关文件。
- 遵守 `windows_subsystem.rs` 的文本契约；需要修改的契约要与规格同步更新。
- API Key 不得写入日志、测试快照或文档。

## 交付
DAO 与迁移、端点解析、切换引擎、路由模块、Tauri 命令、前端组件、单元测试、契约测试更新。
