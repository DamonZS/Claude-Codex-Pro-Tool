# 验收标准：供应商模型映射下拉选择修复

验证对象：`spec/supplier-model-mapping-dropdown.md`

## 验收项

1. 模型映射实际请求模型可下拉选择
   - 通过标准：Claude / Claude Desktop 供应商编辑页的模型映射行中，“实际请求模型”使用 `select` 或等效带下拉箭头控件。
   - 证据：源码断言或手动截图。

2. 下拉选择会写入对应行
   - 通过标准：选择模型时调用 `updateSupplierModelMapping(row.role, "requestModel", value)`，并触发 `modelMappingJson` 与 `modelMapping` 更新。
   - 证据：源码断言与前端类型检查。

3. 标题与列头对应
   - 通过标准：模型映射说明明确展示 `显示名称 / 实际请求模型 / 声明支持 1M`，列头与下方字段顺序一致。
   - 证据：源码断言或手动截图。

4. 选项来源完整
   - 通过标准：下拉选项至少合并已获取模型、当前 `modelList`、默认模型与已有映射值。
   - 证据：源码断言。

5. 构建验证
   - 通过标准：以下命令通过：
     - `cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem supplier_screen_matches_ccswitch_style_layout_and_drag_sorting -- --nocapture`
     - `npm --prefix apps/claude-codex-pro-manager run check`
     - `npm --prefix apps/claude-codex-pro-manager run vite:build`
     - `cargo build -p claude-codex-pro-manager --manifest-path Cargo.toml`
