# 修复 Windows NSIS 安装假成功

## 背景
GitHub Actions run 38361551596 的 Windows installer job 中，Chocolatey 获取 nsis 3.13.0 元数据遭遇 HTTP 504，输出安装 0 个包后仍继续执行。应用编译完成后，安装包步骤因 makensis 缺失失败。

## 目标与范围
- 自动、手动发布使用同一 NSIS 安装检查脚本。
- 安装最多尝试 3 次；两次重试各等待 10 秒。
- 每次检查 Program Files (x86)、Program Files 和 PATH 中的编译器，执行 /VERSION 验证。
- 编译器存在且版本检查成功才将绝对路径导出至 GITHUB_ENV 的 MAKENSIS_EXE。
- 重试耗尽时在安装步骤明确失败，阻止后续应用编译与安装包打包。
- 安装包构建使用已验证的编译器路径，保留原有编译参数与退出码处理。

## 非目标
不调整应用行为、版本号、签名、安装器内容、MSI、macOS、发布资产命名或失败草稿清理规则。

## 用户视角
发布时遇到短暂包源故障会自动重试；持续故障会在依赖安装阶段及时显示错误。

## 技术约束与交付
使用现有 PowerShell 与 Chocolatey，无额外依赖。交付安装脚本、两个发布工作流的调用调整、执行真实工作流安装入口的定向 PowerShell 回归测试及匹配验收文档。