# AGENTS.md

## 项目目的

Claude Codex Pro Tool 是面向 Codex App 与 Claude Desktop 的本地运维控制台。它把 Codex 增强、供应商/Profile 切换、插件与 Skill 管理、记忆辅助、启动器维护、更新工具和 Release 打包整合在一个 Rust + Tauri + React 工作区中。

本项目采用 Harness Engineering 工作方式。AI 编码代理必须基于清晰上下文、明确规格、可验证验收标准和真实验证证据工作，不得一边猜需求一边修改代码。

所有在本仓库工作的 AI 代理，包括 Codex、Claude Code、Hermes Agent、Cursor Agent 或其他代理，都必须优先遵守本文件。

## 目录结构

- `apps/claude-codex-pro-launcher/`：Rust 静默启动器应用及启动器测试。
- `apps/claude-codex-pro-manager/`：Tauri 管理工具。React/Vite 前端位于 `src/`；Tauri Rust 后端位于 `src-tauri/`。
- `apps/claude-codex-pro-mcp/`：MCP 相关应用代码。
- `crates/claude-codex-pro-core/`：核心 Rust 逻辑，包括启动器集成、Claude Desktop 集成、供应商配置、插件中心、记忆辅助、更新/安装流程和桥接逻辑。
- `crates/claude-codex-pro-data/`：Codex 会话、导出和 Provider Sync 的数据访问逻辑。
- `assets/inject/`：注入到 Codex 与 Claude 包装窗口的 JavaScript 资源。
- `scripts/`：安装器、发布和维护脚本。
- `.github/workflows/`：CI、发布和构建产物工作流。
- `docs/`：已有架构、评审、发布和参考文档；`docs/harness-engineering-theory.md` 说明本工作流背后的方法论。
- `spec/`：任务与功能规格文档。任何非平凡实现任务在改代码前都必须有相关 spec。
- `acceptance/`：验收标准文档。每个任务 spec 应尽量有一个对应的 acceptance 文档。

## 必读文档

开始任何开发任务前，必须先阅读并理解：

1. `AGENTS.md`
2. `README.md`
3. `docs/harness-engineering-theory.md`（当任务涉及流程设计、代理分工或工作法演进时）
4. `spec/` 下的相关文件
5. `acceptance/` 下的匹配文件
6. 与任务相关的源码、配置和测试

如果任务没有相关 spec 或 acceptance，必须先创建它们。重要功能、页面、接口、模块或重构项不得直接进入实现阶段。

## 规格文档规则

规格文档是实现目标的定义文档。所有规格文档存放在 `spec/` 下。

建议每个功能或任务一个文件，例如：

```text
spec/feature-user-login.md
spec/feature-order-query.md
spec/feature-admin-dashboard.md
```

一个合格的规格文档必须包含：

- 标题：明确功能或任务名称。
- 背景：业务背景、用户需求、当前问题和目标价值。
- 目标：本次工作包含什么，以及明确不包含什么。
- 用户视角描述：用户如何使用该功能或工作流。
- 功能要求：字段、行为、校验规则、数据更新、权限和边界情况。
- UI / 交互要求：页面结构、按钮行为、提示文案、加载态、空态、错误态、响应式行为和必要的视觉约束。
- 数据与接口要求：输入、输出、数据来源、格式、错误处理和鉴权要求。
- 技术约束：现有框架、依赖、架构边界、兼容性限制和禁止改动区域。
- 交付范围：页面、接口、数据结构、测试、文档、配置和其他预期产物。

## 验收标准规则

验收标准是任务完成的裁判标准。所有验收标准存放在 `acceptance/` 下。

尽量与规格文档保持一一对应关系：

```text
spec/feature-order-query.md
acceptance/feature-order-query.md
```

每个验收标准文档必须说明：

- 它验证的是哪一份规格文档。
- 具体通过/失败标准。
- 必需的验证方式，例如测试、构建、本地运行、手动检查、截图、日志或命令输出。
- 完成任务所需的证据。
- 已知非目标或不在范围内的检查项。

代理在未检查相关验收标准前，不得宣称任务完成。

## 标准任务流程

### 阶段 1：理解任务

- 阅读 `AGENTS.md`。
- 阅读相关规格文档。
- 阅读匹配的验收标准。
- 阅读相关代码、配置和测试。

### 阶段 2：总结上下文

实施前必须总结：

- 当前目标。
- 预计需要修改的文件。
- 不能改动的文件或行为。
- 验收标准。
- 关键风险。

### 阶段 3：实施开发

- 按规格文档开发。
- 严格对照验收标准工作。
- 做最小必要改动。
- 避免无关重构。
- 不引入不必要依赖。
- 不擅自改变架构、生产配置、发布行为或数据位置。

### 阶段 4：验证

汇报完成前，必须至少运行一项真实验证：

- 单元测试或集成测试。
- 类型检查。
- 构建。
- 本地应用运行。
- 有针对性的手动验证。
- 能证明相关行为的日志、截图或命令输出。

不得编造结果。如果无法运行验证，必须说明原因并描述剩余风险。

### 阶段 5：交付

最终汇报必须包含：

1. 任务结论：完成了什么，目标是否满足。
2. 修改内容：修改了哪些文件，每个文件改了什么。
3. 验证结果：运行了哪些命令，结果如何，是否通过。
4. 对照验收标准：哪些项已满足，哪些未满足，以及原因。
5. 风险与后续：剩余风险和可选后续动作。

## 子代理分工建议

复杂工作应使用子代理分工，避免同一个代理同时负责写规格、实现、测试和裁判。

推荐角色：

- 规格代理：阅读需求并生成或更新 `spec/*.md`。
- 验收代理：把规格文档转换为 `acceptance/*.md`，并明确证据要求。
- 实现代理：用最小代码、配置或测试改动实现规格文档。
- 测试代理：根据验收标准验证并报告证据。
- 评审代理：审查是否偏离规格文档、遗漏验收项、存在质量风险或回归。

推荐顺序：

1. 规格代理
2. 验收代理
3. 实现代理
4. 测试代理
5. 评审代理

所有子代理都必须阅读 `AGENTS.md`、相关规格文档和验收标准。不得修改无关内容、跳过验证或在没有证据时宣称完成。

## 构建与验证命令

安装前端依赖：

```bash
cd apps/claude-codex-pro-manager
npm install --package-lock=false
cd ../..
```

启动 Tauri 开发应用：

```bash
cd apps/claude-codex-pro-manager
npm run dev
```

启动仅前端的 Vite 开发服务：

```bash
cd apps/claude-codex-pro-manager
npm run vite:dev
```

交付前建议检查：

```bash
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo fmt --check
cargo test --workspace
cargo build --release
```

常用定向 Rust 检查：

```bash
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml plugin_hub -- --nocapture
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml memory_assist -- --nocapture
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml relay_config -- --nocapture
cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem -- --nocapture
```

优先运行能证明当前任务的最窄验证。若改动涉及共享行为、发布脚本、数据迁移、安全敏感代码或用户可见工作流，应扩大验证范围。

## 安全边界

- 不得删除用户数据、本地 Codex 状态、Claude Desktop 状态或生产配置，除非用户明确要求且任务 spec 覆盖该操作。
- 不得把 API key、Bearer token 或完整授权材料写入日志、文档、测试或记忆文件。
- 不得自动信任第三方 hooks、MCP 安装脚本、插件或下载归档。
- 除非是有文档说明、用户触发、带备份和还原能力的本地补丁流程，否则不得修改官方 Claude Desktop 文件。
- 不得删除或弱化项目 license、作者、仓库、维护者或署名信息。
- 不得回滚无关的工作区改动。

## 最低交付标准

一个回答只有在满足以下条件后才算完成：

- 相关规格文档已存在。
- 匹配的验收标准已存在。
- 如有实现改动，必须遵守这些文档。
- 已运行真实验证，或明确说明无法验证的原因。
- 最终汇报已对照验收标准说明结果。
