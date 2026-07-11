# VibeCoding 大赏系列投稿

> 主题：为开发者做一个本地 AI 运维控制台
>
> 项目：Claude Codex Pro Tool
>
> 适用平台：抖音、小红书、小黑盒

## 使用说明

- 面向开发者撰写，突出工程实践和真实功能。
- 发布前使用真实项目截图替换“配图 3”，并脱敏 API Key、Token、Cookie 和私人路径。
- 不将生成的架构图伪装成真实运行截图。
- 不宣称与 Codex、Claude、OpenAI、Anthropic 或抖音官方存在合作关系。
- 发布前确认活动页面的截止时间和最新投稿要求。

---

## 一、抖音图文版

### 标题

**VibeCoding大赏｜我给开发者做了一个本地 AI 运维控制台**

### 正文

以前用 Codex、Claude Desktop 和各种 MCP 工具时，配置、插件、会话、记忆和更新都散落在不同地方。

所以我做了一个本地 AI 工作台：Claude Codex Pro Tool。

它基于 Rust + Tauri + React，支持多供应商/Profile 切换、Codex 启动增强、Claude Desktop 管理、插件中心、Ponytail、MCP、盘古记忆、Zed Remote、Watcher 自恢复和自动更新。

我比较看重的是“本地优先”：配置和记忆保存在本机，敏感内容会脱敏，安装操作尽量提供预览、备份和恢复入口。

它不是又一个聊天窗口，而是给 AI 开发工具准备的一套控制面板。

`#vibecoding大赏 #ai新星计划 @抖音科技`

### 配图

1. 配置散落 → 本地控制台的 Before / After。
2. 产品功能架构图。
3. 管理工具真实界面截图。

### 视频开场

> 如果你同时在用 Codex、Claude、MCP 和一堆插件，应该会遇到一个问题：工具越来越多，但配置也越来越乱。所以我做了一个本地 AI 运维控制台。

---

## 二、小红书版

### 标题

**我把 Codex、Claude、MCP 和插件装进了一个本地控制台**

### 正文

最近在做一个自己的 AI 开发工具控制台：Claude Codex Pro Tool。

最初的想法很简单——Codex、Claude Desktop、MCP、插件、Skills、供应商配置和历史会话，全部都需要单独管理，时间久了很容易变成：

- 配置文件到处都是。
- API Profile 切换麻烦。
- 插件装了什么记不清。
- 会话历史不好整理。
- 不同 Agent 之间无法共享经验。
- 出问题只能手动查日志。

所以我用 Rust + Tauri + React 做了一个本地管理工具。

目前已经包含：

- Codex 启动增强和本地注入。
- 多供应商 / Profile 管理。
- Responses / Chat Completions 协议适配。
- Claude Desktop 状态、DevTools、新对话管理。
- Claude 中文窗口和可恢复的本地补丁流程。
- 多来源插件中心和安装预览。
- Ponytail、Skills、MCP 管理。
- 盘古记忆：SQLite、本地采集、workspace 隔离。
- MCP 记忆工具：搜索、列表、最近记录、写入。
- Zed Remote 和 Upstream Worktree。
- Watcher 自恢复、日志、更新和 Release。

我现在更喜欢把它理解成：

> AI 编程工具的本地运维控制台。

重点不是堆功能，而是让每个动作都尽量可查看、可确认、可备份、可恢复。

目前项目仍在持续完善，分享的是本地工具设计和工程实践，不代表与任何官方产品存在合作关系。

### 话题

`#AI编程 #Codex #Claude #MCP #Rust #Tauri #VibeCoding #开发者工具`

---

## 三、小黑盒技术版

### 标题

**一个本地 AI 运维控制台是怎么做出来的：Rust + Tauri + React 的工程实践**

### 正文

AI 编程工具越来越多之后，真正麻烦的往往不是调用模型，而是周边运维：

- 供应商和 Profile 如何切换。
- Codex 和 Claude Desktop 如何统一管理。
- 插件、Skills、MCP 如何安装和审查。
- 历史会话如何导出、修复和同步。
- Agent 之间如何共享长期经验。
- Watcher、更新和 Release 如何维护。

Claude Codex Pro Tool 的做法，是把这些能力整合到一个本地 Tauri 管理工具中。

整体结构分为几层：

```text
React 管理界面
        ↓
Tauri Command Layer
        ↓
Rust Core
        ↓
Codex / Claude / SQLite / Git / 本地配置
```

核心 Rust crate 负责启动器、协议转换、供应商配置、插件中心、盘古记忆、远程项目、Watcher 和更新流程。

数据层单独处理 Codex 会话、Markdown 导出、Provider Sync 和备份恢复。

比较有意思的是盘古记忆：

1. 从 Codex 和 Claude 的本地会话中采集可解析文本。
2. 采集时做脱敏、归一化和哈希去重。
3. 记忆按 workspace 隔离。
4. 记忆分为 active 和 archived。
5. 新会话只注入 active 记忆。
6. 通过 MCP 暴露 `memory_search`、`memory_list`、`memory_recent` 和 `memory_learn`。

这样 Codex、Claude Code 或其他 MCP 客户端就能访问同一套本地记忆，而不需要把数据库放到云端。

供应商切换则会同时考虑配置文件、会话数据库和全局状态。执行同步前会加锁、创建备份，失败时尝试恢复，避免只改了一半导致历史会话不可见。

插件中心也不是简单下载后执行，而是把不同来源聚合成统一目录，并根据安装类型展示预览、依赖和风险。社区 MCP 等需要人工确认的内容，不会直接写入配置。

这个项目给我的最大感受是：

> VibeCoding 不只是让 AI 帮忙写代码，也包括把需求、架构、验证、备份和回滚一起做完整。

它目前定位为本地开发者工具，不是 Codex、Claude 或其他官方产品，也不涉及绕过官方限制。所有真实截图都应该先脱敏，尤其是 API Key、Token、Cookie 和本地用户路径。

### 配图

1. Before / After：分散配置 vs 统一控制台。
2. 系统架构图：React → Tauri → Rust Core → 本地数据。
3. 盘古记忆或供应商页面真实截图。

---

## 四、统一配图规划

### 配图 1：Before / After

画面左侧：配置散落、插件分散、会话难找。

画面右侧：一个本地控制台统一管理 Codex、Claude、MCP、插件和记忆。

建议文案：

> 从“到处找配置”到“一个控制台管理”。

### 配图 2：技术架构

```text
Codex / Claude Desktop / MCP
            ↓
      Tauri Manager
            ↓
        Rust Core
     ↙      ↓      ↘
  SQLite   Config   Git/Remote
```

建议文案：

> 本地优先、可审计、可备份、可恢复。

### 配图 3：真实界面

优先选择以下页面之一：

- 供应商 / Profile 页面。
- 插件中心页面。
- 盘古记忆页面。
- 维护与更新页面。

截图前检查：

- API Key 是否隐藏。
- Token 和 Cookie 是否隐藏。
- 用户名、私人路径和项目地址是否脱敏。
- 不要展示未公开的本地数据库内容。

---

## 五、抖音 60 秒视频分镜

| 时间 | 画面 | 旁白 |
| --- | --- | --- |
| 0-5 秒 | Before / After 对比 | 工具越来越多，但配置也越来越乱。 |
| 5-12 秒 | 展示项目启动和概览页 | 所以我做了一个本地 AI 运维控制台。 |
| 12-22 秒 | 展示供应商页面 | 可以管理多个供应商和 Profile，并统一切换。 |
| 22-32 秒 | 展示工具与插件页面 | 插件、Skills、MCP 和脚本也有统一入口。 |
| 32-43 秒 | 展示盘古记忆页面 | 记忆保存在本地 SQLite，还能通过 MCP 共享给不同 Agent。 |
| 43-52 秒 | 展示维护、Watcher、日志页面 | 出问题可以看状态、日志和修复入口。 |
| 52-60 秒 | 展示架构图和项目名 | 这是我用 Rust、Tauri 和 React 做的本地 AI 工作台。 |

## 六、发布前检查

- [ ] 抖音标题以 `VibeCoding大赏` 开头。
- [ ] 抖音正文超过 100 字。
- [ ] 包含 `#vibecoding大赏 #ai新星计划 @抖音科技`。
- [ ] 图文不少于 3 张图。
- [ ] 视频时长不少于 1 分钟。
- [ ] 没有 API Key、Token、Cookie 或私人路径。
- [ ] 没有“破解、绕过限制、无限额度、免费 API、内置 Key、免登录”等表达。
- [ ] 没有未经证实的官方合作或流量承诺。
- [ ] 规格中的规划能力没有被误写成当前已上线功能。
