# 项目消耗总览圆环验收

对应 `spec/overview-project-ring.md`。

1. 项目图为有中心留白的细线圆环，不再是实心扇形；灰色底环和 TOP 彩色分段均可见。
2. SVG 几何采用 100×100 viewBox、半径 42、线宽 10、-90 度起点和分段间约 1.4 单位留白。
3. 圆环分段、TOP 表格和 TOP 占比使用同一份 `projectUsage`；切换 TOP 3/5/10 后数量和分段一致，其他项目显示为灰色底环。
4. 没有项目时显示底环、0 项目且不出现伪造分段。
5. 外层布局、液态玻璃材质、其他卡片及后端采集保持原样。
6. `npm --prefix apps/claude-codex-pro-manager run check`、`npm --prefix apps/claude-codex-pro-manager run vite:build`、`cargo build --release -p claude-codex-pro-manager`、`git diff --check` 运行并记录结果，默认 `target/release` 保留最新程序。
