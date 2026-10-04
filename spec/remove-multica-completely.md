# 彻底移除 Multica

## 背景

CCP 曾移植 Multica 上游的“我的任务”“自动化”“智能体”三个页面，并在 core 里实现了对应的本地工作区、执行存储、webhook、sidecar 托管等后端。三个侧边栏入口已在 `spec/remove-multica-sidebar-features.md` 中下线，但后端、前端移植源码、管理工具 UI、CI 与文档仍整体保留，体积大（约 900 个被跟踪文件、数万行 Rust）且与 CCP 核心定位无关。用户要求把 Multica 从代码中彻底清除。

## 目标

本次包含：

- 删除 `apps/codex-workflow-surface`（Multica 前端移植与适配层）及 core 对其构建产物的 `include_str!` 嵌入。
- 删除 core 中全部 Multica 子系统：`multica*` 模块、`multica_workspace/`、webhook、执行存储、managed runtime / sidecar、skill trust。
- 清理 `routes`、`launcher`、`settings`、`paths`、bridge 等共享文件里的 Multica 路由、trait 方法、设置项和启动逻辑。
- 删除管理工具中的 Multica 连接 / 运行时 / sidecar UI 与全部 `*_multica_*` Tauri 命令。
- 清理 `assets/inject/renderer-inject.js` 中的 Multica 工作区注入逻辑。
- 清理 CI 工作流、安装脚本、`verify-workflow-*` 等脚本中对 workflow-surface 与 Multica 许可文件的构建和暂存步骤。
- 删除 `docs/third-party/multica`、`docs/multica-*.md` 及描述 Multica 功能的历史 `spec/`、`acceptance/` 文档。
- 保留 Codex 页面执行服务 `codex_execution` 的线程 / 回合 / 流式执行能力；但其中的“技能执行请求”（`skill_request`、`CodexSkillExecutionRequest`、`SkillReference`、`resolve_skills` 及相关校验）只被 Multica 路径产生，随 Multica 一并删除，不迁出。

本次不包含：

- 不删除用户本机数据 `~/.claude-codex-pro/multica/`，不在升级时清理它，也不读取它。
- 不改变 Codex 增强、供应商与路由、蒸馏、Computer Use、主题、系统提示词、会话管理等非 Multica 功能的行为。
- 不改写 git 历史。

## 用户视角

升级后：Codex 侧边栏与管理工具中都没有 Multica 相关入口和设置；安装包不再包含 workflow-surface 与 Multica 许可文件；其余功能行为不变；旧版 `settings.json` 中残留的 `multicaWorkspaceEnabled` 等字段不会导致加载失败。

## 功能要求

- 旧配置兼容：`settings.json` 里出现已删除的字段（如 `multicaWorkspaceEnabled`）时，加载与更新都成功；该字段不再被任何代码读取，属于惰性残留。设置更新按原始 JSON 合并并有意保留未知键，所以它不会被自动清除，也无需清除。
- 启动器与 bridge 不再监听或路由任何 Multica webhook / workspace 请求；对这些路径的请求按未知路由处理。
- 管理工具启动与退出不再启动、停止任何 Multica sidecar。
- `codex_execution` 的线程 / 回合 / 流式执行行为不变；技能执行请求相关的字段与方法随 Multica 删除。

## UI / 交互要求

- 管理工具设置页不再有 Multica 相关卡片、连接表单、运行时安装、sidecar 控制。
- `routes.ts` 中旧的 `multica` 路由别名不再保留。

## 数据与接口要求

- 删除 Tauri 命令：`list/save/delete/check_multica_connection`、`get_multica_snapshot`、`start/stop_multica_sidecar`、`get/ensure/cancel/rollback_multica_runtime`、`login/logout_multica_managed`、`set_multica_managed_enabled` 等全部 `*multica*` 命令及其在 `invoke_handler` 中的注册、前端 `types.ts` / `actions.ts` / `tauriPreviewMock.ts` 中的对应项。

## 技术约束

- 分阶段提交，每阶段 `cargo check --workspace` 通过；在 `chore/remove-multica` 分支上完成，验证后再合并 main。
- 会话内不运行会结束真实 Claude 进程的测试（见 core 汉化单测约束），验证以 `cargo check`、定向测试和类型检查为主。
- 不得删除用户数据、本地配置或仓库自身的 license、作者、署名信息。Multica 第三方署名文件随其派生代码一并删除，因为不再分发任何 Multica 派生内容。

## 交付范围

- 代码、脚本、CI、文档的删除与清理；本规格与对应验收文档。
- 重新构建默认 `target/release` 下的最新应用。
