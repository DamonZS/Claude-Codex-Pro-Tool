# 验收标准：删除提示词优化器

验证对象：`spec/remove-prompt-optimizer.md`

## 通过标准

1. 独立提示词优化器前端代码已删除。
   - 通过：前端不存在 `PromptOptimizerCard`、`goPromptOptimizer`、`PROMPT_OPTIMIZER_URL`、第三方提示词优化器仓库链接或服务 URL。
   - 证据：源码检查和 Windows 契约测试。

2. 专属 Tauri 接口已删除。
   - 通过：不存在 `open_prompt_optimizer_window`、`PromptOptimizerWindowPayload`、专属 Payload 构造函数及命令注册。
   - 证据：源码检查、Rust 编译和 Windows 契约测试。

3. 路由和样式残留已清理。
   - 通过：`normalizeRoute` 不再识别 `promptOptimizer`，`LegacyRoute` 不再包含它，专属 CSS 选择器不存在。
   - 证据：源码检查和 Windows 契约测试。

4. 产品说明已同步。
   - 通过：中英文 README、知识图谱说明和 UI 示例不再把提示词优化器列为项目能力；README 功能编号连续。
   - 证据：文档检查。

5. 盘古记忆提示词能力保持不变。
   - 通过：当前项目接续、新项目启动指南及复制给 Agent 的代码仍存在，相关记忆测试通过。
   - 证据：源码检查和 core 记忆测试。

6. 工具与插件页无布局回归。
   - 通过：页面不显示提示词优化器入口或空白占位，其他工具与插件功能正常。
   - 证据：本机启动检查。

7. 检查、测试和默认目录构建通过。
   - 证据：

     ```powershell
     npm --prefix apps/claude-codex-pro-manager run check
     npm --prefix apps/claude-codex-pro-manager run vite:build
     cargo fmt --check
     cargo test -p claude-codex-pro-core --manifest-path Cargo.toml memory_assist -- --nocapture
     cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem -- --nocapture
     cargo build --release
     ```

## 非目标

- 不验收盘古记忆以外的新提示词生成能力。
- 不修改或验证供应商、注入、插件安装、更新和用户数据迁移。
