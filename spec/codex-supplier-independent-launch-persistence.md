# Codex 供应商独立启动持久化

## 背景

供应商切换由管理工具执行，但 Codex 可能随后由桌面快捷方式、官方入口或其他启动方式直接启动。两者必须读取同一份 Codex 配置，否则管理工具显示已配置而独立 Codex 仍使用旧供应商。

## 目标

- 供应商切换将最终 `config.toml` 与 `auth.json` 写入 Codex 官方读取的 home 目录。
- 管理工具、启动器和独立 Codex 对 `CODEX_HOME` 的优先级保持一致。
- 管理工具退出后，不依赖 CCP 进程或 provider sync hook，独立 Codex 仍可读取活动 provider。

## 非目标

- 不修改 Codex 官方运行时或用户现有状态数据库。
- 不在日志、测试输出或界面中暴露 API Key。
- 不要求正在运行的 Codex 进程热加载配置；已启动进程可按 Codex 原生行为重启加载。

## 技术约束

- 继续使用现有原子写入、备份和 `SettingsStore`。
- home 解析顺序必须为 `CODEX_HOME`，再回退 `HOME`/`USERPROFILE` 下的 `.codex`，最后使用平台目录发现。
- 不引入新依赖或新的配置格式。

## 交付范围

- 统一核心 home 解析。
- 持久化回归测试，覆盖管理器写入后独立读取配置的场景。
- 默认 Release 构建与定向测试证据。
