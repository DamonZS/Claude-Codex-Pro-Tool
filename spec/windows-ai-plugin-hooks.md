# 本机 ai-plugins 钩子 Windows 执行兼容修复

## 背景
截图中用户 UserPromptSubmit 完成，插件 UserPromptSubmit/PreToolUse/PostToolUse 反复退出 1。启用的 ai-plugins@claude-plugins-official 有四个 Bash 命令，本机裸 bash 解析为 C:\Windows\System32\bash.exe，WSL ext4.vhdx 缺失，实测退出 1。Git Bash 可运行但脚本调用的 python3 是 WindowsApps 别名，实测退出 49；原脚本 || true 会隐藏此错误。

## 目标与范围
- 仅修复该插件当前 local 缓存的 hooks.json 调用方式，四个原始 .sh 保持原字节。
- 使用本机可用的原生 python 执行包装器，读取原脚本内既有 Python 程序，保持事件输出、API 归因拦截与依赖提示逻辑。
- 使用 stdin 提供原 HOOK_PAYLOAD，保持 UTF-8 中文，设置与原 shell 相同的插件根目录/摘要脚本环境。
- 不改变系统 PATH、WSL、Python 安装、用户 instruction-inject.sh 或其他插件。
- 先备份 hooks.json、记录哈希，提供可恢复原配置的回滚脚本。

## 验收
原 Bash 命令同输入退出 1，修复后四类入口正常退出并输出有效 JSON（需要输出时）；确认 PreToolUse 旧 API 与缺失 agent-id 仍输出 deny，合法 API 不误拦截。恢复备份后原命令仍复现失败，再重施修复。PowerShell 实际外壳与直接进程入口均验证。

## 交付与限制
修复缓存、验证/备份/回滚材料位于 docs/task-evidence/plugin-hooks。插件原脚本不变；插件升级后缓存可能重新生成，此修复仅针对当前已安装 local 版本。真实 UI 历史失败记录不被清除，新钩子行为需以客户端重新加载插件后的新事件确认。
