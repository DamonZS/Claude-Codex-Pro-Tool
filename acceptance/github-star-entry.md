# 验收标准：GitHub 点 Star 入口

验证对象：`spec/github-star-entry.md`

## 通过标准

1. **位置**：`apps/claude-codex-pro-manager/src/components/AppShell.tsx` 中，GitHub Star 按钮的 JSX 出现在「启动/重启 Codex」按钮之前，且两者在同一父容器内。
   - 判据：源码中 `onOpenGithub` 按钮所在行号小于 `onRestartCodex` 按钮行号，父容器相同。
2. **样式一致**：新按钮 `className` 为 `ops-icon-command ops-action-command`，与相邻按钮完全相同。
   - 判据：源码字符串比对。
3. **图标与文案**：使用 `Star` 图标（来自 `lucide-react`），文案为「点个 Star」，`title` 为「在 GitHub 上给 CCP 点个 Star」。
4. **链接行为**：点击触发 `onOpenGithub`，`App.tsx` 传入的回调调用 `openExternalUrl("https://github.com/DamonZS/Claude-Codex-Pro-Tool")`。
   - 判据：`App.tsx` 中出现该 URL 与 `openExternalUrl` 调用，且作为 `onOpenGithub` 的值。
5. **不阻塞**：新按钮不带 `disabled={busy}`。
6. **类型与构建通过**：
   - `npm --prefix apps/claude-codex-pro-manager run check` → 退出码 0。
   - `npm --prefix apps/claude-codex-pro-manager run vite:build` → 退出码 0。
7. **未破坏既有入口**：「启动/重启 Codex」「启动/重启 Claude」「Claude 一键汉化」三个按钮的 `onClick` 与 `title` 保持不变。

## 必需验证方式

- 运行上述两条 npm 命令，记录退出码。
- 用 `Select-String`/Read 检查 `AppShell.tsx` 与 `App.tsx` 的相关行，记录行号作为证据。

## 证据

- 命令输出与退出码。
- 改动后的两处源码片段（含行号）。

## 非目标

- 不验证 GitHub 页面是否真的加载（依赖系统浏览器与网络）。
- 不验证 Star 状态、登录态、埋点。
- 不做真机截图验收（这是纯前端入口，构建通过 + 源码证据即可）。