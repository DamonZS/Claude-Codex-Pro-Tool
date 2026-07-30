# 验收标准：Codex 注入右侧锚点与紧凑控制舱

验证对象：`spec/codex-injection-right-anchor-and-compact-modal.md`

## 验收项

1. 状态入口使用右侧锚点
   - 通过标准：脚本包含右侧锚点查找函数，优先识别最小化控件，并从 Header 右半区识别文件打开或工具按钮。
   - 证据：`injection_script_anchors_status_entry_to_right_titlebar_controls` 测试。

2. 注入入口不覆盖项目名称和原生控件
   - 通过标准：状态入口计算为锚点左侧位置；盘古记忆入口位于状态入口左侧；缺少锚点时使用窗口右侧安全回退。
   - 证据：注入脚本契约测试。

3. 控制舱尺寸紧凑
   - 通过标准：桌面最大宽高不超过 `780px × 540px`，侧栏、Header、Hero 和设置卡片采用紧凑尺寸；内容区保持独立滚动。
   - 证据：`injection_script_uses_compact_pangu_control_deck_layout` 测试。

4. 响应式与功能无回归
   - 通过标准：900px、720px 媒体查询、四个导航入口、设置开关、关闭隔离和服务模式继续存在。
   - 证据：完整 `cdp_bridge` 测试。

5. 构建通过
   - 通过标准：`cargo fmt --check`、完整 `cdp_bridge` 测试和 `cargo build --release` 成功。

## 非目标

- 不修改 Codex 原生标题栏布局。
- 不修改 Manager 主界面或 bridge API。
