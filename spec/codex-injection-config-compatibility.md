# CCP 注入配置的 Codex 兼容性

## 背景

CCP 合并公共设置并写入 Codex `config.toml` 时，可能保留 Codex 已忽略或废弃的字段，导致启动时出现配置警告。截图确证已忽略的字段为配置根或 Profile 中的 `disable_response_storage`、`features.code_mode_tool_description_first` 和 `[features.guardianv2].thread_context`；沿用已有 MCP server 的 `type` 清理。

## 目标

- CCP 清理公共配置和归一化最终配置时删除上述不兼容字段。
- 前端供应商预览和后端导入生成器停止生成 `disable_response_storage`。
- 清理根配置及所有 `profiles.<name>` 覆盖；结构化解析覆盖普通表、dotted key 和 inline table。
- 保留同一配置中的合法 MCP 命令、参数、启用状态、Guardian 的其他设置和其他 feature 字段。
- 对可解析 TOML 使用结构化处理；公共配置原本无法解析时，fallback 仅移除目标 table 中的目标键。
- 保持供应商配置及 `experimental_bearer_token` 持久化行为不回归。

## 非目标

- 不直接修改用户的 Codex 配置文件作为修复前置步骤。
- 不更改 Codex 官方运行时、MCP 命令或其他未报告字段。
- 不在日志或测试输出暴露 API Key。

## 技术约束

- 复用 `relay_config` 现有 TOML 解析、归一化和原子写入路径。
- 不引入新依赖。
- 修改范围限于配置清理和对应回归测试。

## 交付范围

- 公共配置清理与最终配置归一化移除废弃字段。
- 测试覆盖字段移除、合法相邻字段保留及 TOML 可解析性。
- 定向 Rust 测试、格式检查和默认 Release 构建结果。
