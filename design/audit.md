# Codex 注入弹窗审计

target: 用户截图 + `assets/inject/renderer-inject.js`
goal: 消除覆盖并形成 CCP 差异化、可信的本地控制体验

| # | area | issue | sev | fix | eff |
|---|---|---|---|---|---|
| 1 | layering | 遮罩 z-index 为 2147483646，Codex 模型 portal 仍可位于其上 | HIGH | 提升到 2147483647，打开前发送 Escape、失焦并在文档末尾创建独立 stacking context | S |
| 2 | identity | 粗黑描边、纸张网点、Comic Sans 和 POWER PANEL 属于通用漫画模板，难以识别 CCP | HIGH | 改为“盘古本地控制舱”，以本地运行、模型桥接、盘古记忆和安全回退建立品牌语义 | M |
| 3 | hierarchy | 首页是一条连续设置列表，主能力与低频诊断同权 | HIGH | 增加能力概览，并按模型插件、会话工作流、本地运维分区 | M |
| 4 | navigation | 顶部四个胶囊 tab 与内容争夺纵向空间，支持页扩宽导致结构跳变 | MED | 桌面使用固定侧栏场景导航，所有 tab 使用统一宽度 | M |
| 5 | readability | 网点纹理、偏移硬阴影和旋转状态增加长文本噪声 | MED | 使用低对比网格、薄边框和受控光晕，正文卡保持稳定对比 | S |
| 6 | a11y | 关闭按钮约 30px，部分控件缺少统一键盘焦点 | MED | 关闭按钮 40px，增加 focus-visible，窄屏与 reduced-motion 降级 | S |

verdict: 先解决 1-3；它们分别阻断操作、品牌辨识和任务扫描。视觉细节必须服务于“本地控制舱”，而不是再做一套装饰皮肤。
