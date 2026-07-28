# 验收标准：CCP 外观与背景图库

对应规格：`spec/ccp-background-library.md`

## 通过标准

- 主题中心具有“CCP 外观 / Codex 主题”两个互斥视图，内容和操作不混排。
- 切换视图后页面标题、说明、工具栏和数量统计同步切换，不出现另一视图的操作按钮。
- Codex 视图只显示 DIY、制作指南、下载、导入和 Codex 主题；按钮文案为“导入主题”。
- CCP 外观默认卡固定第一张，保存背景以三列卡片展示并可切换。
- 可连续添加至少三张不同背景；重新选择同一图片不会重复创建。
- 当前背景有文字状态；当前项不可删除，切换后原项可删除。
- 恢复默认后图库仍完整存在，再次点击可继续应用。
- 旧单张自定义背景自动迁移为图库项，迁移前后视觉内容和当前状态不丢失。
- 图库状态和日志不包含来源绝对路径；列表使用压缩预览而不是原始 4K 数据。
- 非法格式、低分辨率、超大图片和损坏图片被拒绝且不改变当前背景。
- 新增、应用、恢复和删除失败时当前背景保持不变，无活动暂存残留。

## 验证方式

```powershell
cargo fmt --check
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml codex_theme -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem -- --nocapture
cargo build --release
```

手动验证：

- 在原生 Manager 中切换两个视图，确认标题、工具栏和内容互不混淆。
- 添加两张背景、重复添加其中一张、相互切换、恢复默认并删除非当前项。
- 检查 1920×1080、最小窗口、浅色和深色下卡片布局、滚动和文字可读性。

## 必需证据

- Core 迁移、去重、切换、恢复和删除测试结果。
- 前端类型检查、生产构建和 Windows 契约测试结果。
- CCP 外观与 Codex 主题两个视图截图。
- 默认 `target/release/claude-codex-pro-manager.exe` 的更新时间与大小。
