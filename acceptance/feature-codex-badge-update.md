# 验收：Codex 注入徽标检查更新（对应 spec/feature-codex-badge-update.md）

## 静态检查

| 项 | 通过标准 | 验证方式 |
| --- | --- | --- |
| A1 页面已删 | `assets/inject/renderer-inject.js` 中不再有 `function openClaudeCodexProModal`，且 `document.body.appendChild(overlay)` 的模态构建随之消失；全文 `claude-codex-pro-control-deck` 只允许出现在 CSS 规则里（渲染时不会创建该节点） | `Select-String -Pattern "openClaudeCodexProModal"` 无命中；`node --check` 通过 |
| A2 徽标保留 | `setClaudeCodexProTriggerLabel` 仍写 `CCP ${claudeCodexProVersion}` 的 `aria-label`/`title`/文本，状态点 `claude-codex-pro-backend-indicator` 仍在 | 读函数体确认，未被改动 |
| A3 点击改道 | `configureClaudeCodexProTrigger` 的 click 处理器调用更新流程函数，不再调用旧的模态函数 | 读该函数确认 |
| A4 提示可见 | 新增提示条元素，含「正在检查更新」「已是最新版本」「正在下载」「安装包已启动」「失败」五类文案 | 读新增函数确认文案齐全 |
| A12 开关有家 | 删掉的面板里那 6 个控制在管理器「设置 → 通用」里都有入口：五个 `ToggleRow`（页面功能增强、插件市场解锁、强制解锁入口、特殊插件强制安装、Fast 按钮）+ 服务模式 `Segmented`（标准/兼容增强） | 渲染 `MaintenanceToolsPanel` 断言 5 个 `role="switch"` 与 1 个 `role="radiogroup"`（`InjectionSwitches.test.tsx`） |
| A13 写的是同一份设置 | 点开关调用 `actions.saveSettings({ ...当前设置, 被点的键 })`，其余键原样带回（不重置）；服务模式写 `launchMode` | 同上，断言入参的其它键未变 |
| A14 应用路径已移除 | 「设置 → 通用」里不再有「Codex 应用路径」「Claude 应用路径」两块内容 | `grep "应用路径"` 在 `apps/claude-codex-pro-manager/src` 只剩 `App.tsx` 的运行提示文案 |

## 编译与测试

| 项 | 通过标准 | 验证方式 |
| --- | --- | --- |
| A5 注入脚本语法 | `node --check assets/inject/renderer-inject.js` 退出码 0 | 直接执行 |
| A6 Rust 编译 | `cargo check -p claude-codex-pro-core` 通过 | 直接执行 |
| A7 新路由单测 | `/update/check` 的成功与失败分支各有一条测试通过 | `cargo test -p claude-codex-pro-core --lib routes` |
| A8 既有回归 | `cargo test -p claude-codex-pro-core --lib` 与之前基线相比无新增失败 | 直接执行并对照 |

## 运行时证据（应有）

| 项 | 通过标准 |
| --- | --- |
| A9 真机徽标 | 重启 Codex 后徽标仍显示 `CCP <当前版本>`；点击不再弹出控制台面板，而是显示提示条 |
| A10 更新检查 | 版本已是最新时提示「已是最新版本 <版本>」；把 `version::VERSION` 临时调低（或指向更旧 release）时能提示发现新版本并进入下载 |
| A11 安装启动 | 真机下载安装包后 `launch_installer` 拉起安装程序（与 `update::perform_update` 既有行为一致） |

## 桥接通道（A15–A16，2026-11-02 补）

| 项 | 通过标准 | 证据 |
| --- | --- | --- |
| A15 徽标走的是桥接而不是 HTTP | `renderer-inject.js` 的 `callUpdateBridge` 必须调用 `postJson(path, {})`，不得 `fetch(helperBase + path)`（`helperBase` 指向协议代理端口，没有 `/update/*` 路由，会 404） | `node --check` 通过；`fetch(helperBase` 在注入脚本里只剩 `/backend/status` 那两条既有回退 |
| A16 注入页能真的拿到更新结果 | 通过 CDP 调 `window.__codexSessionDeleteBridge("/update/check", {})` 必须返回 `status: ok` 的 JSON，且 `payload.currentVersion` 与当前安装版本一致 | `cargo test -p claude-codex-pro-core --test bridge_update_live -- --include-ignored --nocapture`：`{"status":"ok","message":"已经是最新版本","payload":{"currentVersion":"V1.70",…,"updateAvailable":false}}` |

> 踩坑记录：第一版用 `fetch("http://127.0.0.1:57321/update/check")` 直连，57321 是 **协议代理**（Codex/Responses 转发）而不是桥接服务；请求打进去只会 404，徽标于是永远提示「检查更新失败」。桥接是注入时通过 CDP 安装的 JS 绑定 `__codexSessionDeleteBridge`，与 `/settings/get`、`/backend/status` 同一条通道。
## 已知非目标

- 不验证 GitHub API 的限流与离线分支以外的网络异常。
- 不验证 macOS 侧徽标（本轮只在 Windows 真机验证）。
- 已排期并完成：删掉面板里的 6 个开关搬进管理器「设置 → 通用」（A12–A13）。原先的「应用路径」两块内容按主人要求整块移除（A14）；「启动/重启 Claude」「Claude 一键开发模式」两个按钮随之消失，但同样的动作仍在顶栏与 App 动作里（`App.tsx` 的 `启动/重启Claude`、`Claude 一键开发模式` 提示标题仍在）。