# 验收标准：Claude、Codex 工具与插件统一资产清单

验证对象：`spec/unified-agent-tools-and-plugin-inventory.md`

## 通过标准

1. 规格与验收文档存在。
   - 通过：两份文档均为 UTF-8 中文可读内容，并覆盖扫描、聚合、切换、仓库状态与检测反馈。
   - 证据：文件检查。

2. 页面只显示一个统一资产面板。
   - 通过：标题为“Claude、Codex 工具与插件”，不再分别渲染“Codex 工具与插件”和“Claude 工具与插件”。
   - 证据：源码检查与页面截图。

3. “同步到当前 Codex”被删除。
   - 通过：统一页面不存在该按钮和对应文案；兼容后端接口是否保留不影响验收。
   - 证据：源码字符串检查与页面截图。

4. Codex MCP 完整发现。
   - 通过：fixture 的 `config.toml` 包含受管和非受管 MCP 时，统一检测全部返回，且启用状态正确。
   - 证据：Rust 回归测试。

5. Codex Skills 完整发现。
   - 通过：fixture 的 Codex skills 根目录包含多个有效 `SKILL.md` 和无效目录时，只返回全部有效技能且不重复。
   - 证据：Rust 回归测试。

6. Claude MCP 聚合多配置路径。
   - 通过：fixture 同时提供普通配置、Claude Code 配置、MSIX 等价候选配置及 `projects.*.mcpServers` 时，全部 MCP 均被发现，同 ID 合并且不重复；关闭同 ID MCP 后，所有配置路径和项目级嵌套均无残留，重新检测保持关闭；注入后续文件写入失败时，已尝试文件恢复原始内容。
   - 证据：Rust 回归测试。

7. Claude Skills 与插件完整发现。
   - 通过：fixture 中有效技能、已安装插件、仅缓存插件能够被区分；仅缓存插件的 Claude 图标不得点亮。
   - 证据：Rust 回归测试。

8. 同一资产跨应用合并。
   - 通过：Claude 与 Codex 均存在同名同类型资产时只返回一行，两个应用状态分别为真；同名不同类型仍为不同资产。
   - 证据：Rust 回归测试。

9. 应用图标准确表达状态。
   - 通过：每行固定显示 Claude、Codex 图标；已启用为亮色，未启用为灰色；图标带可访问名称和悬停说明。
   - 证据：前端源码检查、截图或组件测试。

10. 切换只影响目标应用。
    - 通过：fixture 中点灭 Claude 后 Claude 状态关闭而 Codex 保持不变；反向同理；OpenAI 缓存插件首次启用写入 manifest 中的真实 marketplace ID，不得写成 `plugin@plugins`。
    - 证据：Rust 回归测试或 Tauri 命令测试。

11. 关闭资产不物理删除唯一副本。
    - 通过：Skills/目录型插件关闭后可恢复，重新开启内容一致；测试中原始来源仍存在或可从受管停用位置恢复；嵌套 Skill 往返后相对目录不变。
    - 证据：Rust 回归测试。

12. 仓库状态不再虚报。
    - 通过：只有配置存在、本地来源缺失时不得返回“已解锁/应用已可见”；本地来源存在但运行时不可确认时显示“待应用确认”或等价状态。
    - 证据：Rust 测试与页面文案检查。

13. Claude 仓库状态聚合候选路径。
    - 通过：仓库只写在非首个候选配置路径时，状态仍能识别并返回该实际路径。
   - 证据：Rust 回归测试。

14. Codex 注入包含所有真实本地 marketplace。
    - 通过：fixture 同时提供 OpenAI、第三方、Product Design 和自定义本地 marketplace 快照时，注入清单全部包含；只有配置但缺少本地 `marketplace.json` 的来源不得进入清单，也不得报告应用可见。
    - 证据：Rust 回归测试和注入脚本定向测试。

15. 重新检测有即时反馈。
    - 通过：点击后出现加载态并禁止重复点击；成功显示真实数量摘要；失败显示失败阶段。
    - 证据：源码检查、页面手动验收或组件测试。

16. 操作日志脱敏。
    - 通过：检测与切换会记录动作、资产 ID、目标应用和结果；日志中无 API key、Bearer token、cookie 或完整敏感配置。
    - 证据：定向测试或本地日志查询。

17. 本机真实数据不再明显漏扫。
   - 通过：本机检测结果至少覆盖当前配置中的全部 MCP 和技能根目录中的全部有效 `SKILL.md`；若因去重导致数量减少，统计字段应给出原始发现数、统一条目数和已合并数量。
    - 证据：管理工具结果与只读文件计数对照。

18. 统一面板保留新增 MCP 能力。
    - 通过：MCP 标签存在可访问的“新增 MCP”入口，可选择 Claude 或 Codex，保存后重新检测；页面没有恢复旧的双面板，也没有“同步到当前 Codex”。
    - 证据：前端源码检查、页面手动验收或组件测试。

19. 前端检查与构建通过。
    - 证据：

      ```powershell
      npm --prefix apps/claude-codex-pro-manager run check
      npm --prefix apps/claude-codex-pro-manager run vite:build
      ```

20. 定向 Rust 测试与 manager 构建通过。
    - 证据：

      ```powershell
      cargo test -p claude-codex-pro-core --manifest-path Cargo.toml unified_tool_inventory -- --nocapture
      cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml unified_tool_inventory -- --nocapture
      cargo build -p claude-codex-pro-manager --manifest-path Cargo.toml
      ```

21. 代码评审无阻断缺陷。
    - 通过：独立评审没有 CRITICAL/HIGH；若发现则在交付前修复并重新验证。
    - 证据：评审结论。

22. Windows 后台仓库任务不弹出终端窗口。
    - 通过：管理器启动时自动检测或修复 Codex marketplace、建立 Git 本地快照以及其他工具页后台任务，均通过 `CREATE_NO_WINDOW` 启动外部进程；打开管理器不会出现 `git.exe`、cmd、PowerShell 或 Windows Terminal 弹窗。
    - 证据：Windows 子系统回归测试、Manager 重新构建及本机启动检查。

## 手动验收步骤

1. 打开“工具与插件”。
2. 确认仓库卡不会把单纯配置写入描述为应用已解锁。
3. 点击“重新检测”，观察按钮加载态和完成通知。
4. 在 MCP、Skills、插件三个标签间切换，核对数量与本机文件/配置。
5. 找到同时存在于 Claude、Codex 的资产，确认只显示一行且两个图标独立点亮。
6. 点灭其中一个应用图标，确认另一端不变；再点亮并确认恢复。
7. 在 MCP 标签点击“新增 MCP”，分别核对 Claude/Codex 目标选择与保存后的重新检测。
8. 确认页面不存在“同步到当前 Codex”。

## 非目标

- 不要求本次支持 Claude、Codex 之外的应用。
- 不要求自动安装所有仓库中的可用插件。
- 不要求执行第三方安装脚本。
- 不验证供应商、盘古记忆、汉化注入或发布功能。
