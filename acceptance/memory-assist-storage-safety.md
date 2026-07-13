# 盘古记忆存储位置与写入安全验收标准

对应规格：`spec/memory-assist-storage-safety.md`

## 通过标准

- 旧设置无 `memoryAssistDataDir` 时仍可加载；正式安装目录可识别且可写时默认数据库位于安装盘，开发态安全回退旧目录。
- 用户选择目录后，Manager、Launcher、MCP 的统一 core 解析结果指向同一 `memory_assist.sqlite`。
- 迁移成功后目标数据库通过完整性检查，长期记忆、候选、采集记录数量与源库一致，注入缓存及备份目录按规则迁移。
- 目标存在冲突数据库、目标不可写、完整性校验失败或设置保存失败时，默认路径仍指向源数据库且源数据库可正常打开。
- 迁移不自动删除源数据库，并明确返回重启 Codex、Launcher、MCP 的提示。
- 完全相同的 `record_capture` 重复调用只新增一组事件；字段变化时允许更新记录并新增一组事件。
- 事件裁剪将 `memory_events` 限制为 20,000 条、`memory_activity_events` 限制为 50,000 条，并删除超出保留天数的事件；长期记忆、候选和采集记录不受影响。
- 注入脚本存在 30 分钟、128 条上限、失败可重试的采集指纹去重契约。
- 高级诊断显示当前目录并可选择、迁移；其他导航和页面布局无无关变化。

## 必需验证

```powershell
cargo fmt --check
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml memory_assist -- --nocapture
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml --test cdp_bridge -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo test --workspace -j 2
cargo build --release -j 2
```

## 完成证据

- 定向测试与全量测试、构建的真实退出状态。
- 迁移前数据库备份路径、迁移后完整性检查结果和各核心表记录数。
- `target/release` 中三个应用二进制的路径、大小和更新时间。
- 本地启动 Manager 后的存储目录显示与数据库增长复测结果。

## 已知非目标

- 不要求自动删除 C 盘旧数据库。
- 不要求迁移运行中的旧进程无重启热切换。
- 不要求修改 `settings.json`、日志等小体积应用状态的位置。
