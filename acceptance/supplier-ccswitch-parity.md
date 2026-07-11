# 验收标准：供应商配置与路由系统 cc-switch 对齐

验证对象：`spec/supplier-ccswitch-parity.md`

## 验收项

1. 供应商列表结构对齐
   - 通过标准：供应商卡片包含拖拽柄、图标、名称、官网/Base URL、状态标签、使用/编辑/复制/删除操作。
   - 证据：源码断言或手动截图。

2. 路由开关按应用分组
   - 通过标准：Codex 路由状态独立；Claude 与 Claude Desktop 共用 Claude 路由组；切换过滤器后开关状态正确。
   - 证据：前端源码和回归测试。

3. Claude / Claude Desktop 模型映射完整
   - 通过标准：模型映射包含 Sonnet、Opus、Fable、Haiku、Subagent；列头为显示名称、实际请求模型、声明支持 1M；实际请求模型可下拉写入。
   - 证据：源码断言、类型检查。

4. Codex 多行模型目录完整
   - 通过标准：Codex 模型映射区同时显示“获取模型”和“添加模型”；可添加多行、逐行删除；每行包含菜单显示名、可手输/下拉的实际请求模型、上下文窗口及 cc-switch 示例占位。
   - 通过标准：获取模型只更新候选列表，不自动追加映射行；选择候选只更新当前行。
   - 通过标准：保存时剥离前端 `rowId`，过滤空模型和重复模型，并按界面顺序同步 `codexCatalogJson`、`modelList`、首个 `model` / `testModel`。
   - 通过标准：保存后重新打开可还原全部行；显式保存空数组时不自动补回旧模型。
   - 证据：前端回归测试、Settings 序列化往返测试、类型检查。

5. Codex User-Agent 预设完整
   - 通过标准：User-Agent 可自由输入；“预设”下拉包含 cc-switch 源码中的 5 个固定值，选择后写入输入框且可继续编辑。
   - 通过标准：预设弹层不会越过管理工具窗口边界。
   - 证据：前端源码断言、手动检查。

6. cc-switch 导入保留关键字段
   - 通过标准：导入配置保留 API Key、targetApp、apiFormat、routeEnabled、claudeDesktopMode、modelMappingJson、websiteUrl、configContents、authContents。
   - 通过标准：Codex `settings.modelCatalog.models` 的显示名、实际模型、上下文窗口和顺序进入 `codexCatalogJson`，有效模型 ID 同步进入 `modelList`。
   - 证据：Rust 测试。

7. 后端路由语义对齐
   - 通过标准：Claude / Claude Desktop 代理按安全路由 ID 映射上游模型，1M 后缀不会发给上游，Fable 未配置时回落 Opus，Subagent 不被默认映射误伤。
   - 证据：`protocol_proxy` 定向测试。

8. 构建验证
   - 通过标准：以下命令通过：
     - `npm --prefix apps/claude-codex-pro-manager run check`
     - `npm --prefix apps/claude-codex-pro-manager run vite:build`
     - `cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem supplier_screen_matches_ccswitch_style_layout_and_drag_sorting -- --nocapture`
     - `cargo test -p claude-codex-pro-core --manifest-path Cargo.toml --test model_catalog -- --nocapture`
     - `cargo test -p claude-codex-pro-core --manifest-path Cargo.toml --test cdp_bridge injection_script_unlocks_custom_model_catalog -- --nocapture`
     - `cargo test -p claude-codex-pro-core --manifest-path Cargo.toml protocol_proxy -- --nocapture`

## 非范围

- 不要求迁移 cc-switch 全量依赖。
- 不要求改变管理工具整体色调。
- 不要求重置用户配置或删除现有数据。
