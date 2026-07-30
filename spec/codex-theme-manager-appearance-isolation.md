# Codex 主题与 Manager 外观隔离

## 背景

Manager 在没有用户专门选择 CCP 背景时，会从当前 Codex 主题读取背景资产并渲染到管理工具壳层，导致应用 Codex 主题后管理工具同步换肤。这违反两套外观相互独立的产品边界。

## 目标

- 应用、切换或恢复 Codex 主题只影响 Codex Renderer。
- Manager 只显示用户在“CCP 外观”中明确选择的背景。
- 未选择 CCP 背景时，Manager 使用自身浅色、深色或跟随系统外观。
- 保留用户已保存、已选择的 CCP 背景数据与操作能力。

## 功能要求

- `active_manager_background` 只读取 `current_manager_background_id` 对应的 Manager 背景库项目。
- Codex 主题的 `asset_data_uris` 不得作为 Manager 背景回退来源。
- Codex Theme Loader 继续在 Codex Renderer 中使用主题资产。
- 切换 Codex 主题不得修改 Manager 背景选择；清除 Manager 背景不得修改 Codex 主题。

## 技术约束

- 不删除主题包资产，不改变 Codex Theme Loader。
- 不修改 Manager 背景图库的导入、选择、清除和删除接口。
- 不新增依赖，不回滚其他工作区改动。

## 交付范围

- `crates/claude-codex-pro-core/src/codex_theme.rs`
- 本规格及对应验收文档。
