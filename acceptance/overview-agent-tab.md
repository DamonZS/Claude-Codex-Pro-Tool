# 概览双视图与 Agent 概览验收

对应 `spec/overview-agent-tab.md`。

1. 初次进入概览显示总览；双按钮可互相切换且不修改外层 UI。
1a. 概览组件不渲染旧供应商路由、当前请求链路和智能诊断 DOM；无需依赖 CSS 隐藏，供应商管理页仍可使用。
2. Agent 概览按三指标、Agent 轨道、趋势、上下文、消耗明细顺序呈现；没有安全评估卡。
3. 选择 Agent、时间范围、模型/项目时，展示由真实快照重新聚合的结果；无数据或字段缺失不展示示例数值。
4. Agent 轨道可滚动；趋势带柱线、坐标、范围；上下文可展开；明细带用量条和空态。
5. Skill 覆盖由各 Agent 本地根目录及标记文件实测得出；多根目录同名去重、深度限制和符号链接跳过有测试；未安装、失败、不支持有不同呈现。
6. 扫描以用户主目录而非 `.claude` 子目录为基准；WorkBuddy 存在 `.workbuddy` 安装目录且无用量事件时仍显示绿色状态点、排列在未安装 Agent 前；已安装 Agent 组内按 Token 排序。
7. Agent 概览能纵向滚动到上下文构成和消耗明细；卡片与轨道使用总览同级透明玻璃材质，不改外层布局或主题。不以浏览器预览或截图替代应用构建。
8. `npm --prefix apps/claude-codex-pro-manager run check`、数据层定向测试、概览契约测试、Vite 构建、`cargo build --release -p claude-codex-pro-manager` 与 `git diff --check` 通过，默认 `target/release` 中有最新可执行文件。
