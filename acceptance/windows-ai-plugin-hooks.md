# Windows ai-plugins 钩子验收

对应 spec/windows-ai-plugin-hooks.md。

1. 原 UserPromptSubmit / PreToolUse / 两个 PostToolUse 命令均真实执行并退出 1，stdout 含 WSL ERROR_PATH_NOT_FOUND。
2. 修复后的四个命令用完全相同输入退出 0、stderr 空，提示 JSON 的 hookEventName 与事件匹配。
3. 旧 endorctl api、缺少 agent-id 两种 API 违规仍输出 permissionDecision=deny；带 agent-id 的合法调用不误阻止。
4. 依赖安装提示保留，manifest PostToolUse 按既有程序输出 {}，普通不相关提示和非法 JSON 不引入崩溃。
5. 四个原 .sh 的 SHA-256 修改前后一致；hooks.json 可解析，备份和回滚可重建原字节。
6. 真实 hooks.json 回滚后用同一输入仍退出 1；随后重施修复。
7. 单独使用 powershell.exe 外壳调用更新的四个命令成功，支持 stdin 中文输入。

实际用户客户端的历史失败统计不是本地脚本回归的清理目标；本轮不自动关闭客户端。
