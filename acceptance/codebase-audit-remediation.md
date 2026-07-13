# 全量代码审查缺陷修复验收标准

对应规格：`spec/codebase-audit-remediation.md`

## 通过标准

- 更新命令不再接收完整 `Release`；后端重新读取固定索引并校验预期版本。
- Core 测试证明不可信 scheme、host、仓库路径和安装器名称会被拒绝，合法平台安装器可通过。
- 隐藏凭据时 `experimental_bearer_token` 不出现在预览；生成配置控件不可编辑。
- 隐藏凭据时 Header/Body 请求覆盖中的授权字段不明文显示，控件保持只读。
- Unix 测试证明原子写和备份文件权限为 `0600`，相关目录为 `0700`。
- Claude 汉化提权命令中不存在向 `S-1-5-32-545` 授予递归 Modify 的参数。
- Windows 和 macOS 发布契约均断言 MCP 二进制随包分发并纳入签名/卸载路径。
- 自动发布契约断言 tag SHA 校验、取消清理和稳定 release URL，且拒绝 `/untagged-`。
- `repair_backend` 错误分支返回失败；启动命令使用 blocking pool。
- 设置草稿和记忆搜索均有防止旧响应覆盖新状态的请求版本保护。
- 设置刷新与保存共享单调请求版本，较早发起的响应不能覆盖较晚操作。
- Manager 会话删除回归测试使用临时备份目录，在用户应用状态目录不可写时仍可独立验证候选数据库回退和去重删除。
- 注入契约覆盖长词优先、幂等 guard、模型弹窗负向锚点和 iframe load 隐藏 fallback。
- 注入回归测试连续触发同一会话 DOM mutation/`scanDeferred` 至少 100 次时，同一指纹最多发起一次 `/memory/capture`；在途重复被合并，消息、会话或工作区变化后各允许一次新采集，失败后可按有界退避重试。
- Core 测试对同一规范化采集做串行、并发和重新打开数据库后的重放，最终仅有 1 条 `memory_captures`、1 条 `memory_events.capture_recorded` 和 1 条 `memory_activity_events.capture`，且重复调用不刷新无业务变化的时间戳。
- 故障注入测试证明采集记录及两类事件在同一事务中提交；任一事件写入失败时不留下部分成功，之后重试仍得到唯一完整结果。
- 固定时钟测试证明 `memory_events` 删除 30 天以前记录并最多保留按 `created_at DESC, id DESC` 排序的最新 20000 条；恰好位于 30 天边界的记录保留。
- 固定时钟测试证明 `memory_activity_events` 删除 90 天以前记录并最多保留按 `created_at DESC, id DESC` 排序的最新 50000 条；恰好位于 90 天边界的记录保留。
- 保留清理在数据库打开/迁移和持续追加事件时均会触发，重复清理结果一致；清理前后 `memory_items`、`memory_candidates`、`memory_captures` 的行数和内容不变，长期记忆候选生成与采集测试仍通过。
- 新安装路径测试证明 `memory_assist.sqlite` 及 SQLite sidecar/迁移/备份位于安装盘的可写持久数据目录，而不是旧 `~/.claude-codex-pro` 默认目录；默认目录不可写时返回错误且不创建第二份库。
- 共享解析器测试在默认目录和用户自选目录两种场景下分别启动或调用 Manager、Launcher、MCP，三者报告规范化后的同一数据库路径；仅旧库存在时三者共同使用旧库并报告待迁移，不在新目录创建空库。
- Manager 路径交互测试覆盖当前路径展示、目录选择取消、不可写/非法目录、迁移中禁用、成功待重启、目标冲突、占用和失败状态；取消或失败不更改共享路径。
- 成功迁移测试使用包含 WAL 未 checkpoint 已提交记录的旧库，确认执行 `VACUUM INTO`，源/目标 `PRAGMA integrity_check` 均为 `ok`，schema/user_version 兼容，且 `memory_items`、`memory_candidates`、`memory_captures` 行数与内容一致。
- 原子迁移测试证明目标最终文件只会从不存在变为完整数据库，随后共享路径一次性切换；迁移成功后旧源库仍存在并被标记为非活跃，界面明确提示重启 Manager、Launcher、MCP/helper，重启前不恢复跨进程写入。
- 目标冲突测试预先放置任意 `memory_assist.sqlite`，迁移必须失败，冲突文件字节哈希、旧库字节哈希和共享路径设置均不变；不得覆盖、合并或截断目标。
- 迁移故障矩阵至少覆盖源完整性失败、目标不可写、迁移锁被占用、`VACUUM INTO` 失败、目标完整性失败、原子 rename 失败和共享设置提交失败。每种失败都保留旧库及旧活动路径，不删除长期记忆、待确认候选或采集记录，并且不留下可被运行时误选的第二份活跃库。
- 不出现无关 UI 重构或用户数据格式变化。

## 必需验证

```powershell
cargo fmt --check
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml memory_assist -- --nocapture
cargo test --workspace -j 2
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
node scripts/release/verify-release-workflow.js
$env:CARGO_BUILD_JOBS='2'
npm --prefix apps/claude-codex-pro-manager run build
```

## 完成证据

- 上述命令的真实退出状态。
- 注入去重、Core 并发幂等、30 天/20000、90 天/50000 四组自动化测试的测试名、断言计数和通过输出。
- 迁移成功及故障矩阵的自动化输出，包括源/目标完整性检查、关键表迁移前后计数、冲突文件前后哈希、失败后活动路径；证据不得包含记忆正文。
- 默认与自选目录场景下 Manager、Launcher、MCP 报告的规范化路径，以及成功迁移后的“需要重启”界面截图或 UI 契约断言。
- `target/release` 中 `claude-codex-pro.exe`、`claude-codex-pro-manager.exe`、`claude-codex-pro-mcp.exe` 的路径、大小和更新时间。
- 最终差异复审结果，以及仍需真实平台或线上发布验证的剩余风险。

## 已知非目标

- 不要求真实发布 GitHub Release。
- 不要求在本机具备 macOS 签名与 DMG 构建环境。
- 不要求自动操作或覆盖用户真实 Claude Desktop 安装。
- 不要求自动删除成功迁移后保留的旧源库；旧库清理由用户后续明确触发。
- 不要求对 `memory_items`、`memory_candidates`、`memory_captures` 应用事件表的时间/数量保留策略。
