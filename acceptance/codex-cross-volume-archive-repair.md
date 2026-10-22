# 本机 Codex 跨磁盘归档修复验收

对应 spec/codex-cross-volume-archive-repair.md。

## 当前准备阶段
1. 从原生日志确认 thread/archive 的 Windows os error 17，并只读核对指定会话的 rollout_path、archived 状态。
2. 默认脚本返回 preview，真实 sessions 保持普通 C 盘目录，现有归档联接仍指向 H 盘。
3. 在 C 盘临时目录创建 sessions、将 archived_sessions 联接到受控 H 盘临时目录；原生 MoveFileExW（不启用跨盘复制回退） 应返回 17。
4. 在同一临时布局执行修复脚本，逐文件哈希一致且原始目录备份保留；再次 MoveFileExW（不启用跨盘复制回退） 成功。
5. 将测试文件在 H 盘恢复至修复前状态后执行回滚；源目录恢复普通 C 盘目录，原始哈希一致，再次 MoveFileExW（不启用跨盘复制回退） 返回 17。
6. 目标已存在时停止；PowerShell AST、git diff --check 通过。

## 实际维护阶段（待确认）
- 用户明确确认关闭 Codex 及迁移约 255 个会话，保留 C 盘原始备份。
- 执行脚本 Apply 后核对所有会话文件 SHA-256、目录联接及现有归档目录不变。
- 用户重启 Codex 后通过原生归档操作确认成功；不得把临时文件验证当作真实会话已归档的证据。
