# 验收标准：CCP 设置直接读写

验证对象：[spec/settings-direct-write.md](../spec/settings-direct-write.md)。

## 通过标准

1. `SettingsStore::save` 直接覆盖传入路径的设置文件，不调用 `atomic_write`，
   且保存后能由 `SettingsStore::load` 读取。
2. `SettingsStore::update` 直接覆盖正式设置文件，不创建以 `.tmp` 结尾的
   设置临时文件；未知 JSON 字段仍被保留。
3. 缺失文件仍加载为默认设置；格式错误的现有 JSON 仍不会被局部更新覆盖。
4. Codex 供应商的 API Key、Base URL 和 `upstreamBaseUrl` 经保存、读取后仍
   按当前 `authContents`、`configContents`、`upstreamBaseUrl` 规则恢复。
5. `atomic_write` 及其非 SettingsStore 调用点不因本任务改变。
6. 直接写入仍创建缺失父目录并保留现有私有权限设置。

## 验证证据

- 运行 SettingsStore 相关的定向 Rust 测试，其中包含直接写入、局部更新、
  供应商凭据往返和错误输入保护。
- 运行 `cargo fmt --check`。
- 运行 `cargo test -p claude-codex-pro-core settings_store -- --nocapture`。

## 非范围

- 不要求对 Codex、Claude、主题、插件和备份文件的原子写入路径进行变更。
- 不要求生成安装包或重启应用。
