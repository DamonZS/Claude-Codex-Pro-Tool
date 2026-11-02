# Codex 注入徽标：删掉控制台页面，点击改成检查更新

## 背景

CCP 在 Codex 窗口里注入了一个徽标（`.claude-codex-pro-trigger`，文本 `CCP <版本号>`，带一枚后端状态点），点击它会弹出「本地能力控制面」（`.claude-codex-pro-control-deck` 模态：主页 / 推荐内容 / 支持 / 合作请联系微信 四个标签页，内含增强开关、插件解锁、服务模式等）。

主人对这块的判定（原话）：「把这个codex的注入页面删掉吧，没什么用，把注入标识和版本号留着就行了，点击就是检查更新，有更新就下载」。

## 目标

1. 删除注入的「控制台」页面（整个模态及其标签页内容），Codex 窗口里不再出现这个面板。
2. 保留徽标：仍是 `CCP <版本号>` + 后端状态点，位置与样式不变。
3. 徽标点击改为：检查更新 → 有新版本就下载并启动安装包；同时就地给出可见的文字反馈（检查中 / 已是最新 / 正在下载 / 已启动安装）。

## 非目标

- 不改徽标的注入位置、尺寸、配色与状态点语义。
- 不改 `settings.json` 里已有的开关值（删掉界面不重置它们的持久化值）。
- 不做自动后台轮询更新（只有点击才检查）。
- 不动 Claude 一侧的注入徽标（`claude-chinese-inject.js`）。

## 用户视角

Codex 窗口右上角仍有一枚 `CCP 1.67` 样式的小条。点它：先看到「正在检查更新…」，若已是最新则提示「已是最新版本 1.67」；若有更新则显示「发现 1.68，正在下载…」，下载完成后提示「安装包已启动」，系统安装程序随之弹出。全程不再弹出控制台面板。

## 功能要求

- 徽标点击触发一次更新检查；检查期间重复点击不叠加请求（用 busy 标记）。
- 检查结果来自 `GET https://api.github.com/repos/DamonZS/Claude-Codex-Pro-Tool/releases/latest` 语义（复用 core 的 `update::check_for_update`）。
- 有更新时立刻下载当前平台的安装包并启动安装（复用 core 的 `update::perform_update`，它已做资产挑选、大小校验、`launch_installer`）。
- 失败时显示一句人话（例如「检查更新失败：网络不可用」），不抛异常、不刷控制台错误。
- 提示条 4 秒后自动消失；不可遮挡 Codex 自身的主要操作区（右上角徽标下方定位）。

## 数据与接口要求

新增两个桥接路由（`crates/claude-codex-pro-core/src/routes.rs` 的 `handle_bridge_request`，与 `/settings/get` 同级）：

- `POST /update/check` → `{ status: "ok", message, payload: { currentVersion, latestVersion, releaseSummary, assetName, assetUrl, updateAvailable } }`
- `POST /update/install` → `{ status: "ok", message, payload: { version, installerPath, launched } }`；失败返回 `{ status: "failed", message }`

两者都由注入脚本通过 `helperBase`（`http://127.0.0.1:57321`）+ `x-claude-codex-pro-token` 头调用。

## 技术约束

- `assets/inject/renderer-inject.js` 是手写单文件（448KB，`include_str!` 进 core），无构建步骤；行尾是混合的（多数 CRLF，两行 LF）⇒ 改动必须按字符串锚点做，不按行号重排。
- 删除范围必须只包含 `openClaudeCodexProModal` 函数本体（顶层，第 3005–3280 行），不能碰到它的邻居（`selectClaudeCodexProTab`、`findNativeMenuInsertionPoint`、`configureClaudeCodexProTrigger`）。
- 桥接服务器与 Codex 窗口同进程权限：更新检查与下载都在 Rust 侧做，注入脚本不直接访问网络。
- 不新增依赖。

## 交付范围

- `crates/claude-codex-pro-core/src/routes.rs`：两个新路由 + 两个辅助函数 + 单测（成功/失败各断言一次状态字符串）。
- `assets/inject/renderer-inject.js`：删除模态构建函数；徽标点击改调更新流程；新增提示条与 `checkClaudeCodexProUpdate`。
- 本文档与 `acceptance/feature-codex-badge-update.md`。