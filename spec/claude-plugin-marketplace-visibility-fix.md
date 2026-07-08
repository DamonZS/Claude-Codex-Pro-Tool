# Claude 插件仓库可见性修复

## 背景

管理工具「工具与插件」页显示 Claude 官方插件仓库和 Ponytail 插件仓库已写入，但用户在 Claude Desktop 的插件目录中看不到对应仓库。现场排查发现当前 Claude 1.19367 前端资源中使用的配置字段是 `allowedPluginMarketplaces`，而管理工具仍写入并检测旧字段 `extraKnownMarketplaces`，导致管理工具误判为已安装。

同时，运行中的 Claude 会写回活跃 `Claude-3p/claude_desktop_config.json`，可能覆盖掉管理工具刚写入的旧字段或非活跃路径，进一步造成“管理工具显示成功，Claude 实际不可见”。

## 目标

本次要完成：

- Claude 插件仓库修复时写入 Claude 新版识别的 `allowedPluginMarketplaces` 数组。
- 保留旧字段 `extraKnownMarketplaces` 作为兼容信息，但不能再仅凭旧字段判定成功。
- 状态检测以 `allowedPluginMarketplaces` 中的 GitHub 仓库条目为准。
- 修复动作写入前关闭 Claude Desktop，避免运行中的 Claude 把活跃配置写回并覆盖。
- 保留写入多个可能 3P 配置路径的能力。

本次不包含：

- 不自动点击 Claude UI 内的“添加市场”或“安装插件”确认。
- 不删除 Claude 账号、缓存、偏好、插件数据或数据库。
- 不改动 Codex 插件仓库逻辑。
- 不改动 Claude 中文注入或本机汉化逻辑。
- 不引入外部依赖。

## 用户视角描述

用户点击「修复 Claude 插件仓库」后，管理工具关闭 Claude Desktop，写入 Claude 官方和 Ponytail 插件仓库到新版可识别配置中，并提示需要重新启动 Claude Desktop 后查看插件目录。重新进入 Claude 插件目录时，应能在组织/Provisioned 相关区域或添加仓库流程中读取这些仓库。

## 功能要求

- 写入格式：

```json
{
  "allowedPluginMarketplaces": [
    { "source": "github", "repo": "anthropics/claude-plugins-official" },
    { "source": "github", "repo": "DietrichGebert/ponytail" }
  ]
}
```

- 对已有数组去重合并，不删除用户手工添加的其它仓库。
- 同时保留旧 `extraKnownMarketplaces` 兼容块。
- 状态检测必须检查 `allowedPluginMarketplaces`。
- 只有旧 `extraKnownMarketplaces` 而没有新字段时，状态应显示未配置/需修复。
- 修复动作应在写入前尝试关闭 Claude Desktop。

## UI / 交互要求

- 不调整页面布局。
- 修复完成提示要说明已写入新版可见配置，并提示重启 Claude Desktop。
- 管理工具卡片中的状态不再因旧字段而误报成功。

## 技术约束

- 最小改动集中在 Claude Desktop marketplace 配置读写。
- 不新增依赖。
- 不改动发布脚本。
- 不删除用户数据。

## 交付范围

- 规格文档。
- 验收标准文档。
- 核心配置写入与状态检测修复。
- Rust 单测与结构测试锚点。
- 本机配置检查、测试和 manager debug 构建。
