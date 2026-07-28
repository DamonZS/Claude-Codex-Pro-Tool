# CCP 液态玻璃应用与安装图标

## 背景

当前 CCP 应用、桌面快捷方式和安装程序使用黑色不透明方形图标。图标在 Windows 桌面、任务栏和安装器中显得沉重，缩小后主要表现为黑色方块，和管理工具当前的液态玻璃视觉系统不一致。

## 目标

- 保留 CCP 现有的双环品牌识别。
- 将应用图标改为透明圆角玻璃底、液态高光和清晰双环的视觉样式。
- 同步 Windows 应用、Windows 安装器、macOS 应用包和 README 使用的图标资源。
- 保证 16px、24px、32px、48px、128px 和 256px 等常见尺寸下仍可识别。

## 非目标

- 不修改应用名称、安装目录、快捷方式名称或启动行为。
- 不修改 Tauri、NSIS 或 macOS DMG 的打包流程。
- 不改变 CCP 的业务逻辑、配置或用户数据。

## 视觉要求

- 图标保持居中的外环与内环结构，延续暖金与青蓝的品牌关系。
- 外部四角必须是真实透明区域，不得继续使用不透明黑色方形底。
- 主体采用克制的液态玻璃效果：半透明磨砂底、边缘折射、顶部高光和轻量纵深。
- 轮廓在浅色和深色背景上都可辨认，不依赖单一背景颜色。
- 不加入文字、字母、水印或复杂细节。

## 资源与接口

- `apps/claude-codex-pro-manager/src-tauri/icons/icon.png`：1024x1024 RGBA 主图标，同时作为 macOS `.icns` 输入。
- `apps/claude-codex-pro-manager/src-tauri/icons/icon.ico`：包含多种 Windows 常用尺寸的图标容器。
- `assets/images/claude-codex-pro.png` 与 `assets/images/claude-codex-pro.ico`：README 和公共资源副本。
- `docs/images/claude-codex-pro.ico`：文档资源副本。
- 现有 Launcher build script、Tauri 配置、NSIS 安装脚本和 macOS 打包脚本继续引用上述资源，不新增第二套打包路径。

## 技术约束

- PNG 必须保留 Alpha 通道，并包含透明角落与半透明玻璃像素。
- ICO 至少包含 16、24、32、48、64、128 和 256 像素帧。
- 所有分发副本必须来自同一主图标，避免应用、快捷方式和安装器视觉不一致。
- 图标资源不得包含本地路径、用户数据或第三方受限素材。

## 交付范围

- 更新 PNG 与 ICO 图标资源。
- 验证 Windows Launcher、Manager 和 NSIS 安装器继续使用新图标。
- 验证 macOS 打包脚本可继续由主 PNG 生成 `.icns`。
- 重建默认 `target/release` 应用，并在可用时生成 Windows 安装包。
