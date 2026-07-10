# 验收标准：概览页远程公告配置

验证对象：`spec/overview-announcement-card.md`

## 验收项

1. 远程配置文件存在
   - 通过标准：`assets/config/announcement.json` 是有效 JSON，包含当前默认公告完整字段。

2. 主仓库双源读取
   - 通过标准：默认 URL 指向主仓库 GitHub Raw 与 jsDelivr 的同一公告文件，并继续附加缓存破坏参数。

3. 远程公告覆盖兜底
   - 通过标准：远程列表包含 `official-toporeduce-api` 时保留远程标题、正文、按钮和链接，不再被去重逻辑删除。

4. 离线兜底
   - 通过标准：所有远程来源失败时仍返回一条当前默认公告。

5. 前端动态渲染
   - 通过标准：`OverviewScreen` 从 `AdsResult` 渲染公告字段，源码中不再硬编码具体正文和目标链接。
   - 通过标准：概览路由调用 `load_ads`，刷新概览会重新获取公告。

6. 安全边界
   - 通过标准：远程公告按 React 文本节点渲染，不使用 `dangerouslySetInnerHTML`；外链继续调用现有 `openExternalUrl`。

7. 验证与构建
   - 通过标准：core ads 测试、Manager 回归测试、前端类型检查、Vite 构建和 Manager debug 构建通过。
