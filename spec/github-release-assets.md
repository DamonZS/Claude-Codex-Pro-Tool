# GitHub Release 构建与资产发布修复

## 背景

GitHub Actions 的 `Auto release installers` 当前发布结果不符合目标：Release 页面缺少规范的“更新内容 / 验证”说明，Assets 数量不足 9 个；截图中的构建还出现 runner 未获取和内部错误，导致发布链路失败。

## 目标

- 自动 Release Notes 必须包含“更新内容”和“验证”两个区块。
- Release 页面最终应有 9 个 assets：
  - macOS arm64 DMG
  - macOS arm64 ZIP
  - macOS x64 DMG
  - macOS x64 ZIP
  - Windows x64 setup EXE
  - Windows x64 ZIP
  - latest.json
  - Source code zip
  - Source code tar.gz
- 自动发布 workflow 需要上传 Windows ZIP、macOS ZIP、DMG、installer 和 latest.json。
- latest.json 继续排除 latest.json 自身，记录可下载安装资产。
- workflow 结构测试必须覆盖 Release Notes 与 9 个 assets 约束，防止回归。

## 非目标

- 不修改应用业务逻辑。
- 不更改版本号策略。
- 不删除既有 release-assets 手动发布工作流。
- 不引入第三方发布服务。

## 技术约束

- 只修改 GitHub workflow、发布相关测试和必要文档。
- Windows/macOS 构建脚本仍使用现有 Rust、Vite、NSIS、macOS packaging 流程。
- 保持 `V0.xx` tag 规则。
