# Claude 开发配置新增式写入

## 背景

Claude Desktop 第三方开发配置保存在 `Claude-3p/configLibrary`。当前管理工具始终使用固定 profile ID `00000000-0000-4000-8000-000000157210`。现场检查确认该 ID 同时被 CC Switch 使用，因此管理工具写入开发配置时会覆盖 CC Switch 的 profile 文件和 `_meta.json` 条目。

## 目标

- Claude“一键开发模式”和供应商写入统一改为新增式 profile 写入。
- 保留 CC Switch、Claude UI 和其他工具创建的所有 profile 文件及 `_meta.json` 条目。
- 不同供应商写入不同 profile；同一供应商重复写入保持幂等。
- 写入后将 `appliedId` 指向本次新增或更新的 CCP profile，使配置立即成为待重启生效的选中配置。

## 非目标

- 不删除、迁移或改写现有固定 ID profile。
- 不清理孤立 profile。
- 不修改 Claude 汉化文件。
- 不在本次任务中真实写入用户当前 Claude 配置；只修改后续按钮行为。

## 功能要求

- profile ID 根据供应商名称和规范化 Base URL 稳定生成，使用 CCP 专属 UUID 前缀。
- profile ID 不包含 API Key、Token 或模型列表。
- 相同名称和 Base URL 必须得到相同 ID；不同供应商必须得到不同 ID。
- `claude_desktop_config.json` 只合并更新 `deploymentMode`，保留其他字段。
- `configLibrary/_meta.json` 保留所有非本工具字段和条目，只对当前 CCP profile 条目执行追加或同 ID 更新。
- 新增 profile 后允许更新 `appliedId`，但不得删除此前选中的 profile 文件或条目。
- profile 文件已存在时保留未知字段，只更新本工具管理的 gateway、认证、代理地址和模型字段。
- 兼容 Claude Desktop / Microsoft Store 路径中带 UTF-8 BOM 的合法 JSON；解析时忽略文件开头的 BOM，写回时允许规范化为无 BOM UTF-8，但必须保留全部既有 JSON 字段。
- 无供应商参数时只开启开发模式外壳，不删除现有 profile 或 meta 条目。
- 切回官方部署模式时只写入 `deploymentMode=1p`，不删除任何第三方 profile 或 meta 条目。
- 备份、快照和回滚必须覆盖本次实际写入的动态 profile 路径。

## UI / 交互要求

- 成功提示使用“已新增开发配置”或“已更新本工具开发配置”，不得继续暗示覆盖全部配置。
- 现有按钮位置和调用命令保持不变。

## 技术约束

- 不引入新依赖。
- ID 算法必须跨进程稳定。
- 不在日志、测试、文档中记录真实 API Key。
- 主要修改 `claude_desktop_provider.rs`、`plugin_hub.rs` 和相关测试。

## 交付范围

- 动态 profile ID 与新增式写入逻辑。
- 供应商预览、应用、开发模式状态与恢复逻辑适配。
- Rust 回归测试、Manager 检查与构建。
