# Codex 供应商配置一致性

## 背景

部分用户在使用 CCP 切换或删除 Codex 供应商后恢复会话时，Codex 报告活动 provider 不存在。该问题来自 `config.toml` 根级 `model_provider` 与 `model_providers` 表生命周期不同步。

## 目标

- 每次 CCP 写入 `config.toml` 后，活动 `model_provider` 必须指向实际存在的 provider table。
- 删除当前自定义供应商后，优先切换到剩余 provider；没有 provider 时清除活动引用。
- 支持动态 provider ID，不将逻辑限定为 `custom`。
- 不改变用户的非供应商配置项，不读取或记录密钥。

## 技术约束

- 使用现有 `toml_edit` 解析和原子写入流程。
- 解析失败时拒绝写入，避免生成半成品配置。
- Codex Home 继续由 `default_codex_home_dir()` 统一解析，尊重 `CODEX_HOME`。

## 验收

- 活动 provider 缺表时写入后自动修复或清除孤立引用。
- 删除当前 provider 后配置可被 Codex 解析。
- 动态 provider ID 切换保持根键和表键一致。
