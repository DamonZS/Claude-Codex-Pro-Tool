# 验收：维护、设置与 Codex 注入精简

对应 spec/settings-maintenance-injection-pruning.md。

1. 维护页固定禁用开关、三个禁用路径按钮和重复修复入口不存在；有效检查/修复/Watcher/Claude 动作保留。源码契约测试验证。
2. 设置页修复后端不存在，矩阵不含列出的九类移除项；供应商独立展示且没有同步开关。源码契约测试验证。
3. 客户端摘要不再把固定关闭的删除/导出/移动功能显示为会话增强。源码契约测试验证。
4. 四个旧字段分别单独开启时注入判定为 false；真实时间线开启时为 true。本地工作流仍保留独立触发。Rust 单元测试验证。
5. BackendSettings::default 与空 JSON 反序列化的供应商默认均为 true，高风险及废弃开关均为 false；显式旧配置值继续读取。Rust 单元测试验证。
6. 前端 check、生产构建、Manager windows_subsystem 和定向 core 测试通过；修改文件格式检查通过。
7. 默认 target/release 下生成新的 Manager 应用，记录路径、时间、大小与 SHA256。构建输出证明。
8. 原始副本、修改副本、差异、验证记录和可执行回滚脚本保留在忽略的 docs/task-evidence 下；相同命令/输入验证 baseline、modified、rollback，恢复哈希等于原始。

不涉及已有用户配置值的强制迁移、官方 Codex/Claude 当前版本的现场注入验证或其他页面重构。UI 交付以源码契约、类型检查和真实构建为证据，不用浏览器预览替代桌面构建。
