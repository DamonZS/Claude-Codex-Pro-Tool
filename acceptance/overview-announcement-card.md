# 验收标准：概览页远程公告配置

验证对象：`spec/overview-announcement-card.md`

## 验收项

1. 远程配置文件存在
   - 通过标准：`assets/config/announcement.json` 是有效 JSON，包含根级 `enabled: false` 和当前默认公告完整字段。

2. 主仓库双源读取
   - 通过标准：默认 URL 指向主仓库 GitHub Raw 与 jsDelivr 的同一公告文件，并继续附加缓存破坏参数。

3. 远程开关覆盖内置配置
   - 通过标准：远程 `enabled: true` 时显示远程公告；列表包含 `official-toporeduce-api` 时保留远程标题、正文、按钮和链接，不再被去重逻辑删除。
   - 通过标准：远程 `enabled: false` 时返回空公告列表，不因内置配置开启而重新显示。
   - 通过标准：远程配置缺少 `enabled` 或字段类型错误时按关闭处理。

4. 离线兜底
   - 通过标准：所有远程来源请求或解析失败时读取当前安装版本内置配置。
   - 通过标准：当前内置配置默认关闭并返回空公告列表；未来版本把内置 `enabled` 改为 `true` 后可显示内置公告。

5. 前端动态渲染
   - 通过标准：`OverviewScreen` 从 `AdsResult` 渲染公告字段，源码中不再硬编码具体正文和目标链接。
   - 通过标准：概览路由调用 `load_ads`，刷新概览会重新获取公告。
   - 通过标准：`enabled` 为 `false` 或公告列表为空时不渲染公告卡片。

6. 安全边界
   - 通过标准：远程公告按 React 文本节点渲染，不使用 `dangerouslySetInnerHTML`；外链继续调用现有 `openExternalUrl`。

7. 注入推荐与公告同源
   - 通过标准：注入脚本仅请求主仓库 GitHub Raw 与 jsDelivr 的 `assets/config/announcement.json`，不再请求旧广告仓库。
   - 通过标准：远程读取成功时展示配置中的标题、正文、按钮文案、链接和亮点；远程失败时读取构建时嵌入的同一份配置。
   - 通过标准：`enabled: false` 时管理工具公告与注入推荐均不展示推荐卡。

8. 联系入口与可读性
   - 通过标准：管理工具“关于”页和注入弹窗均显示“合作请联系微信”。
   - 通过标准：注入联系页继续显示 QQ 群 `10061615`、`1076215359` 及一键添加入口。
   - 通过标准：注入 QQ 群标签和群号具有专用高对比文字样式，在深色控制舱背景上可读。

9. 验证与构建
   - 通过标准：core ads 测试、Manager 回归测试、前端类型检查、Vite 构建、workspace 测试和默认 `target/release` Release 构建通过。
