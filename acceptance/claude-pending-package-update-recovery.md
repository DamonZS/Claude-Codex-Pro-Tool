# 验收：Claude 待完成包更新的启动恢复

对应 spec/claude-pending-package-update-recovery.md，继承 claude-desktop-launch-window-recovery 的进程、窗口与真实 Release 验收要求。

1. 官方更新恢复只在当前启动失败且没有 Claude 进程时触发。定向测试覆盖进程存在、启动成功、无拒绝码三个跳过分支。
2. PowerShell 查询限定当前用户 Claude、当前操作开始时间、AppXDeploymentServer 404、错误码 0x80070005、同一包名/发布者及更高版本。脚本契约测试覆盖这些条件。
3. 官方注册命令使用 RegisterByFamilyName 与 ForceTargetApplicationShutdown，不使用全应用关闭、不执行 WindowsApps exe、不写官方文件与用户数据。
4. 注册恢复最多一次；恢复失败保留原始启动失败，恢复成功重走原计划并且继续验证真实窗口。定向测试覆盖成功、失败、未形成窗口。
5. 必须运行 cargo test -p claude-codex-pro-core claude_desktop_launch -- --nocapture 及 git diff --check。
6. 最终由主任务构建默认 Release。现场启动仍受系统包注册权限影响，记录官方恢复命令的退出状态、注册版本及真实 Claude PID/窗口；现场验证未通过时明确列为未完成，不用模拟测试替代。
