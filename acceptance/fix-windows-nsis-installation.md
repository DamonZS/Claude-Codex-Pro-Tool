# Windows NSIS 安装检查验收

对应规格：spec/fix-windows-nsis-installation.md。

## 通过标准
1. Chocolatey 返回 0 但编译器缺失时继续重试，3 次后抛错。
2. Chocolatey 首次非零、第二次安装成功时通过；首次返回 0 但工具缺失、第二次成功时也通过。
3. 首次安装成功时仅执行一次，MAKENSIS_EXE 导出真实编译器路径。
4. 版本探测持续失败时重试 3 次后抛错，不导出路径。
5. 自动、手动发布均满足以上场景，并从 MAKENSIS_EXE 执行安装器编译。
6. 既有发布契约验证通过，应用、资产及 macOS 发布行为保持原有规则。

## 验证与证据
- pwsh -NoProfile -File scripts/release/test-windows-nsis-installation.ps1：用隔离环境文件和替代命令执行工作流中的实际安装入口；替代 choco 模拟 504 假成功及恢复，成功探测使用本机 pwsh /VERSION，失败探测使用固定退出码脚本。它验证重试与门禁，不表示真实 NSIS 已安装。
- node scripts/release/verify-release-workflow.js。
- git diff --check；PowerShell AST 解析；工作流 YAML 解析。
- 原失败日志及 BASELINE / MODIFIED / ROLLBACK 验证记录保存在 docs/task-evidence/windows-nsis/。

## 实机验收限制
本地回归不连接 Chocolatey、不安装软件、不生成安装包。完整安装包及 Release 结果以新 GitHub Actions run 为准。