# CCP 液态玻璃应用与安装图标验收标准

对应规格：`spec/liquid-glass-app-icons.md`

## 通过标准

1. 主 PNG 为 1024x1024 RGBA 图像，四角 Alpha 为 0，并同时包含半透明和不透明主体像素。
2. 图标保留清晰的暖金外环与青蓝内环，在浅色、深色和透明棋盘背景上均可辨认。
3. Windows ICO 至少包含 16、24、32、48、64、128 和 256 像素帧，256 像素帧保留透明度。
4. `apps/claude-codex-pro-manager/src-tauri/icons/`、`assets/images/` 和 `docs/images/` 中的分发副本保持一致。
5. Launcher build script、Tauri 配置和 NSIS 安装器仍引用 `src-tauri/icons/icon.ico`。
6. macOS DMG 脚本仍以 `src-tauri/icons/icon.png` 生成 `.icns`。
7. 默认 `target/release/claude-codex-pro.exe` 和 `target/release/claude-codex-pro-manager.exe` 完成重新构建。
8. 若本机存在 NSIS，则重新生成 Windows 安装包；若不存在，明确报告未执行安装包打包。
9. `cargo fmt --check`、相关 Windows 契约测试和 `git diff --check` 通过。

## 验证证据

- PNG 尺寸、颜色模式和 Alpha 统计。
- ICO 帧尺寸列表。
- 1024px 主图标及 16/24/32/48px 小尺寸合成预览。
- Release 构建命令与产物修改时间。
- Windows 安装包路径或 NSIS 不可用说明。

## 非验收项

- 不要求在 Windows 环境生成 macOS `.icns` 或 DMG。
- 不要求修改桌面上已经存在的快捷方式缓存；重新安装或刷新图标缓存后显示新图标。
