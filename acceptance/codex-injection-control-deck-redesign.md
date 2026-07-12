# 验收标准：Codex 注入盘古控制舱重设计

验证对象：`spec/codex-injection-control-deck-redesign.md`

## 通过/失败标准

1. **模型菜单不再覆盖**
   - 遮罩 z-index 为 `2147483647`。
   - 打开弹窗前触发 Escape 关闭临时菜单并让当前活动元素失焦。
   - 弹窗仍在 `document.body` 末尾创建，具有独立 stacking context。

2. **形成 CCP 差异化视觉**
   - 存在 `claude-codex-pro-control-deck` 主题标识。
   - 可见文案包含 `PANGU LOCAL CONTROL DECK`、`本机运行`、`模型桥接`、`盘古记忆` 和 `可审查回退`。
   - 不再包含 `claude-codex-pro-comic-shell`、`POWER PANEL`、漫画字体和旧漫画主题 token。

3. **信息架构清晰**
   - 场景导航和工作区形成桌面双栏布局，窄屏退化为单列。
   - 首页包含“模型与插件通道”“会话与工作流”“本地运维与诊断”分区。
   - 工作区可独立滚动，Header、关闭按钮和导航不随长列表消失。

4. **功能无回归**
   - home、recommendations、support、contact panel 保留。
   - 设置开关、后端状态与修复、服务模式、支持和联系入口保留。
   - 关闭按钮仍阻止事件传播，遮罩关闭后立即 return。

5. **响应式与可访问性**
   - 窄屏存在明确媒体查询。
   - 交互控件有 `focus-visible` 样式。
   - 存在 `prefers-reduced-motion` 降级规则。

## 必需验证

```powershell
cargo fmt --check
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml --test cdp_bridge injection_script_uses_pangu_control_deck_theme -- --nocapture
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml --test cdp_bridge injection_script_modal_close_does_not_toggle_settings -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
```

## 完成证据

- 定向契约测试输出。
- 类型检查和前端构建输出。
- 本地 Codex 截图，确认模型菜单未覆盖且控制舱在常见窗口尺寸可用。

## 非目标检查

- 不要求修改管理工具主界面。
- 不要求修改 Codex 原生模型菜单。
- 不要求新增后端 API。
