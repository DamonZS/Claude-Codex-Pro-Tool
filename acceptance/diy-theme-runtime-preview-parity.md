# 验收标准：DIY 主题预览与 Codex 运行时一致性

对应规格：`spec/diy-theme-runtime-preview-parity.md`

## 通过标准

### 运行时 CSS

- 全屏 DIY CSS 包含 78% 自动背景色工作区表面、96% 自动表面色 Composer 和完全不透明侧栏。
- 全屏根画布继续引用 `var(--ccp-theme-art, none)`，不包含额外全屏渐变，相关表面 `backdrop-filter` 为 `none`。
- 全屏原生首页图标使用 `50%` 圆角。
- 上方长条主视觉为 `min(88%, 470px)` x `108px`，使用 `center / cover no-repeat`、`8px` 圆角和预览同款阴影。
- 中央大卡片主视觉为 `min(76%, 360px)` x `138px`，使用 `center / contain no-repeat`、`8px` 圆角和预览同款阴影。
- 长条和卡片的主图通过 Chromium 可计算的独立背景长属性加载，`getComputedStyle(...).backgroundImage` 不是 `none`。
- 三种布局的主视觉顶部相对 `[role="main"]` 均为 64px；长条标题位于主视觉下方，卡片增高后标题同步下移，Composer 的底部位置不变。
- 三种布局的首页标题字重均为 450。

### PNG 预览

- 三种布局均生成有效的 960x600 PNG，且输出彼此不同。
- 全屏 PNG 的侧栏为不透明表面，主工作区和 Composer 的采样像素分别符合 78% 背景表面和 96%内容表面的预期混合结果。
- 上方长条与中央大卡片的图片边界、宽高比例和位置与 720x410 实时预览按 960x600 基准等价，不再使用旧的 620x150 与 390x196 尺寸。
- 上方长条保持 `cover` 裁切，中央大卡片保持 `contain` 完整显示。
- 上方长条与中央大卡片使用 8px 圆角，并绘制与实时预览 `0 10px 24px rgb(0 0 0 / 0.18)` 等价的软阴影。

### 下载与导入主题隔离

- DIY 活动载荷不包含 `CCP light-theme runtime compatibility` 标记，也不包含该兼容层的全屏 `main:has(...)` 背景规则。
- 浅色下载或普通导入主题仍追加现有浅色运行时兼容层。
- 非 DIY 主题的活动载荷仍读取原始 `entry_style`；官方 Dream Skin 兼容替换继续通过现有测试。
- 本次修改不触碰 Theme Loader、下载源、主题包清单或非 DIY 主题文件。

### 回归与真实运行态

- 已保存的 DIY 主题无需重新保存，重新获取活动载荷或重启 Codex 后使用新模板。
- 在真实 Codex 主 Renderer 中分别应用全屏、上方长条和中央大卡片主题，检查 `getComputedStyle`：背景图被实际引用、主视觉尺寸与预览一致、标题字重为 450、Composer 表面清晰。
- 三种真实 Codex 截图与工作台预览在主图尺寸、裁切、工作区透明度和输入区层次上无明显差异。

## 必需验证

```powershell
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml diy_image_layouts_generate_distinct_css_and_previews -- --nocapture
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml diy_runtime_payload_keeps_download_theme_compat_isolated -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo fmt --check
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml -- --nocapture
```

若本机真实 Codex、CDP 或主题背景测试夹具不可用，必须明确列出未完成的运行态证据，不得用仅通过源码或单元测试代替。

## 失败条件

- 真实 Codex 中任一布局仍使用旧尺寸或旧透明度。
- 浅色 DIY 的 banner/card 图片仍被额外铺成全屏背景。
- 为修复 DIY 而修改共享 Loader 或下载主题 CSS，或使非 DIY 主题载荷发生变化。
- 只修改实时预览而没有修改运行时 CSS 和 PNG，或只修改运行时而主题卡片继续使用旧比例。
- 未观察到回归测试在修复前失败、修复后通过。

## 非目标

- 不验证远程主题仓库可用性或重新下载官方主题。
- 不修改 DIY 工作台布局、字段和保存交互。
- 不要求更改供应商、会话、记忆、插件、模型、汉化或发布流程。
