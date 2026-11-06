# GitHub 点 Star 入口

## 背景

CCP 的仓库是 `https://github.com/DamonZS/Claude-Codex-Pro-Tool`。用户反馈希望在主界面的操作区直接放一个 GitHub 入口，点一下就跳到仓库点赞（Star），不用再翻设置页或 README。

当前主界面头部操作区（`apps/claude-codex-pro-manager/src/components/AppShell.tsx`）依次是「启动/重启 Codex」「启动/重启 Claude」「Claude 一键汉化」三个按钮。用户希望新入口放在「启动/重启 Codex」**左边**。

## 目标

- 在「启动/重启 Codex」按钮左侧新增一个 GitHub Star 入口按钮。
- 点击后用系统默认浏览器打开 CCP 的 GitHub 仓库页面。
- 不新增窗口、不改现有三个按钮的位置与行为。

不包含：登录 GitHub、Star 状态检测、自动点赞、埋点统计。

## 用户视角

1. 打开 CCP 主界面，头部操作区最左侧多出一个「点个 Star」按钮。
2. 点击它，系统浏览器打开 `https://github.com/DamonZS/Claude-Codex-Pro-Tool`。
3. 用户在该页面点击 Star 完成点赞。

## 功能要求

1. 按钮位于 `AppShell` 头部操作区，排在「启动/重启 Codex」之前（同一行、同一组）。
2. 按钮复用现有按钮样式类 `ops-icon-command ops-action-command`，与相邻按钮视觉一致（图标 16px + 文案）。
3. 图标使用 `lucide-react` 的 `Star`；文案为「点个 Star」。
4. `title` 提示为「在 GitHub 上给 CCP 点个 Star」。
5. 点击行为：调用现有的 `actions.openExternalUrl`（Tauri 命令 `open_external_url`），参数为仓库地址常量。
6. 按钮不受 `busy` 状态禁用（与启动/重启类操作不同，打开浏览器与本地任务互不阻塞）。
7. `type="button"`，避免在表单上下文里触发提交。

## UI / 交互要求

- 不使用新的 CSS 类，避免样式分叉。
- 文案与现有按钮长度接近，不得把头部操作区挤换行。

## 数据与接口要求

- 新增 `AppShell` 的 prop：`onOpenGithub: () => void`。
- `App.tsx` 渲染 `AppShell` 时传入 `() => void openExternalUrl("https://github.com/DamonZS/Claude-Codex-Pro-Tool")`。
- 不新增 Tauri 命令。

## 技术约束

- 只改 `apps/claude-codex-pro-manager/src/components/AppShell.tsx` 与 `apps/claude-codex-pro-manager/src/App.tsx`。
- 不新增依赖；`lucide-react` 已在使用。
- 不改动现有按钮的样式类、顺序与其他 prop。

## 交付范围

- 前端两处改动。
- 本 spec 与对应 acceptance 文档。
- 验证：`npm --prefix apps/claude-codex-pro-manager run check`、`npm --prefix apps/claude-codex-pro-manager run vite:build`。