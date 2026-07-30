# 主题背景方向兼容与下载菜单层级

## 背景

部分 JPEG 使用 EXIF 方向标记，照片应用显示为 `1920×1200`，底层像素尺寸可能记录为 `1200×1920`。Manager 按固定宽高校验时会误报尺寸不足。Codex 主题下载菜单还会被后续主题卡片的层叠上下文覆盖，导致菜单内容显示不完整。

## 目标

- Manager 背景尺寸按长边和短边校验，兼容带方向标记的横向照片。
- Codex 主题下载与导入菜单始终显示在主题卡片、标签和状态行上方。
- 保持菜单滚动、按钮禁用和主题下载行为不变。

## 功能要求

- 图片长边至少 `1280`、短边至少 `720` 即通过尺寸校验。
- `1280×720`、`1920×1200` 与原始存储为 `1200×1920` 的图片均通过。
- 页面提示与错误文案使用长边/短边描述。
- 下载菜单通过 React Portal 挂载到 `document.body`，并使用原生模态 `dialog` 进入浏览器 Top Layer，脱离主题卡片和 WebKit 合成层。
- Top Layer 菜单按触发按钮坐标固定定位，并在滚动、缩放时更新位置；点击透明 backdrop 或按 Escape 均可关闭。
- Top Layer 菜单根据根节点明暗主题使用完全不透明底色，不允许下方主题卡片透出或覆盖菜单。
- “下载主题”触发器必须复用 Manager 的标准描边按钮组件，下载箭头与文字保持单行、水平和垂直居中，并显示完整边框与背景。
- Codex 主题标题区右侧的刷新、DIY 主题、制作指南、下载主题和导入主题操作组必须与左侧标题信息在桌面布局中垂直居中；窄屏布局继续按现有换行规则排列。

## 交付范围

- `crates/claude-codex-pro-core/src/codex_theme.rs`
- `apps/claude-codex-pro-manager/src/components/CodexThemeCenterScreen.tsx`
- `apps/claude-codex-pro-manager/src/workspace.css`
- Manager 源码契约测试、规格和验收文档。
