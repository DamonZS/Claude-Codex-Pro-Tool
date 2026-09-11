# 验收：Windows MSI Release 资产

对应规格：`spec/windows-msi-release-asset.md`

## 通过标准

1. 两条 Windows Release 工作流均安装 WiX Toolset。
2. 两条 Windows Release 工作流均执行 Tauri MSI bundle，并检查 `target/release/bundle/msi/*.msi` 存在。
3. 手动发布直接上传 MSI；自动发布将 MSI 纳入 Windows artifact，并由发布 job 上传。
4. 自动发布的资产数量校验为 7，`latest.json` 生成逻辑会读取并收录 MSI。
5. 工作流 YAML 语法、Tauri CLI 参数和文档检查通过。

## 验证方式

- 使用本地 Tauri CLI `2.11.4 --version` 检查 CLI 可用。
- 检查两条工作流的 MSI 步骤、上传 glob 和数量校验。
- GitHub Windows runner 上实际安装 WiX 后运行完整工作流，确认 MSI 文件上传成功。

## 当前执行记录

- 两条工作流已加入 WiX 安装、MSI 构建、文件存在检查和上传路径。
- 本机没有 Chocolatey/WiX，未在 Windows 本地生成 MSI；GitHub Windows runner 是最终打包验证环境。
- `latest.json` 使用 Release API 的全部 assets 动态生成，新增 MSI 会自动进入列表。
