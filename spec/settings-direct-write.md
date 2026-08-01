# CCP 设置直接读写

## 背景

CCP 的供应商资料由 `SettingsStore` 持久化到用户目录下的
`.claude-codex-pro/settings.json`。当前 `SettingsStore::save` 和
`SettingsStore::update` 经由临时文件和替换操作写入。用户要求该配置改为
直接覆盖正式 `settings.json`，并从该正式文件直接读取。

## 目标

- `SettingsStore::save` 直接写入其 `path` 指向的正式设置文件。
- `SettingsStore::update` 在完成既有读、合并和规范化逻辑后，直接写入其
  `path` 指向的正式设置文件。
- `SettingsStore::load` 继续直接从正式设置文件读取，保留缺失文件返回默认
  设置、无效 JSON 返回默认设置的既有兼容行为。
- 保存后，供应商资料及 API Key / Base URL 的现有恢复逻辑保持不变。

## 非目标

- 不改动 `atomic_write` 公共辅助函数，也不改变 Codex `config.toml`、
  `auth.json`、主题、插件、Claude 配置或其他文件的写入策略。
- 不迁移、清理或重写用户已有的 `settings.json` 以外的数据。
- 不改变设置文件的 JSON 字段、脱敏规则、权限策略、锁策略或前端/Tauri
  命令协议。

## 用户流程

1. 用户在供应商页面保存资料，CCP 将规范化后的完整设置直接写入
   `settings.json`。
2. 用户重启或刷新管理工具，CCP 直接读取同一个 `settings.json` 并显示
   原有供应商资料。
3. 用户执行局部设置更新时，未知字段仍会被保留，已知字段按现有合并规则
   更新。

## 功能要求

- 直接写入前仍须创建缺失的父目录，并保留私有目录/文件权限设置。
- 直接写入错误必须向调用方返回，不能报告保存成功。
- 同进程写入锁和设置文件锁继续覆盖 `save`、`update` 的完整写操作，防止
  并发局部更新丢失字段。
- 不创建、替换或遗留 `settings.json.*.tmp` 临时文件。
- `save` 与 `update` 输出仍为格式化 JSON。

## 技术约束

- 仅修改 `crates/claude-codex-pro-core/src/settings.rs` 及必要的定向测试。
- 不新增依赖。
- 现有对 `atomic_write` 的其他调用必须保留。

## 交付范围

- SettingsStore 的直接读写实现。
- SettingsStore 直接写入、局部更新和供应商资料往返的回归测试。
- 与本规格匹配的验收文档。
