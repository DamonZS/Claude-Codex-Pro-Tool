# macOS 浅色主题可读性与 Claude Desktop 路由关闭

## 背景

已安装的 `Codex Dream Skin - macOS` 在当前 Codex Renderer 中能成功注入浅色背景，但主题包未声明新版 Codex 使用的前景色、控件和 VS Code 兼容变量，导致导航文字与主内容继续使用白色系颜色，页面表现为近乎全白。供应商页关闭 Claude Desktop 路由时会先恢复官方部署模式，但成功分支提前返回，没有保存已经计算出的 `routeEnabled: false`，因此提示关闭后设置仍保持开启。

## 目标

- 已安装的旧版浅色 Codex 主题无需重新下载，即可在生成运行时载荷时获得稳定的深色前景色与浅色控件变量。
- 深色主题及默认主题载荷保持现有行为。
- Claude Desktop 活动路由关闭成功后，同时撤销运行配置并持久化关闭状态。
- 任一步失败时不显示完整成功提示。

## 功能要求

- `data-ccp-theme-shell="light"` 主题载荷追加受控兼容 CSS，覆盖当前 Codex 使用的 `--color-text-*`、`--color-token-*`、输入、菜单、列表和 `--vscode-*` 前景色变量。
- 浅色主题首页背景继续使用稳定的 Codex 首页指纹定位，不依赖可能变化的 `main.main-surface` 类名；主题声明了 `--ccp-theme-art` 时，当前首页主区域应显示该资产而不是纯白底。
- Codex 26.721 在首页布局前新增了可为空的直接子节点；Dream Skin 不得再把 `[role="main"] > div:first-child` 当作首页布局。运行时必须把旧锚点升级为直接包含 `[data-feature="game-source"]` 的首页布局节点，避免空节点被撑满视口并遮住真正的首页。
- 已安装的 `Codex Dream Skin - macOS` 旧包无需重新下载；重新生成活动主题载荷后，应恢复参考效果中的头图、品牌标识、首页标题、建议卡片和底部输入区。Windows Dream Skin 使用同一 DOM 结构时同步获得该兼容修正。
- 兼容 CSS 使用主题自身的 `--ccp-theme-*` 变量并提供可读的浅色回退值，不读取或修改供应商配置。
- 非浅色主题不追加该兼容层。
- 关闭活动 Claude Desktop 路由时，恢复官方部署模式成功后保存全部目标 Profile 的 `routeEnabled: false`、Direct 模式元数据和配置内容，然后才提示完成。
- Claude Desktop 本地代理的模型目录、健康检查和 Messages 入口必须读取活动 Profile 的 `routeEnabled`；关闭后不得继续向第三方上游转发尚未重启的 Claude Desktop 请求。
- 路由关闭后，本地模型目录返回空列表、健康检查返回未就绪，Messages 请求返回明确的“路由已关闭，请重启 Claude Desktop”错误。
- 路由设置保存失败时保留已有失败提示，不额外提示“已关闭”。

## 交付范围

- `crates/claude-codex-pro-core/src/codex_theme.rs`
- `apps/claude-codex-pro-manager/src/screens.tsx`
- `crates/claude-codex-pro-core/src/protocol_proxy.rs`
- Manager 契约测试、Core 主题测试及对应验收文档。
