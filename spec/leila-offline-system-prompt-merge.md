# 系统提示词页集成 Leila Codex 部署

## 背景

用户要求将 `F:\迅雷下载\Leila-Codex-Offline-1.0.7-windows-python38-314-AC\Leila-Codex-Offline-1.0.7-windows-python38-314-AC` 中的系统提示词页面与功能合并到本项目的系统提示词页面；目标包当前是 Electron 37.2.6 打包产物，已提取出 `main.js`、`preload.js`、`renderer/app.js`、`renderer/styles.css` 和资源目录，没有可直接编译的源工程。

## 目标

- 在本项目现有 `SystemPromptScreen` 中承载目标包已确认的提示词浏览、启用、编辑、导入、同步和环境状态能力。
- 复用现有 Tauri/Core 的存储、原子写入、备份、外部修改检测和恢复语义。
- 保持本项目统一导航、背景特效、颜色和交互风格；目标包视觉仅作为信息架构参考。
- 保持 `model`、`provider`、注入和供应商配置不变。
- 提供显式目标 `.codex` 目录的 Leila 资源部署、SHA-256 校验和失败回滚。
- 提供 Windows Python x64 环境检测，并通过隐藏 PowerShell 子进程从动态 pip 源安装固定版本依赖。
- 页面加载只检测状态；部署和回滚必须由用户显式触发。

## 非目标

- 不把目标包中的 Electron 主进程代码直接复制到 React/Tauri。
- 不迁移目标包的 `ac` 或其他不可信提示词内容作为实现指令。
- 不修改 Claude Desktop 官方文件，不改变供应商路由或默认模型。
- 在没有目标包可构建源码和重打包链前，不声称已生成新的 Leila EXE。
- 不覆盖 `hooks/instruction-inject.sh`，保留其现有 opt-in 语义。

## 功能设计

1. **页面入口**：系统提示词作为一级路由；保留分类筛选、搜索、内置/自定义区分和当前状态摘要。
2. **提示词卡片**：显示标题、来源、更新时间、启用状态、托管/外部修改/孤儿标记；支持预览、编辑、删除、启用和停用。
3. **应用模式**：`preserve` 只切换提示词指针并保存原配置；`replace` 写入托管 Markdown 并替换指针。两者都必须原子写入并保留回滚信息。
4. **导入/同步**：仅接收 UTF-8 `.md`，大小上限 1 MiB；远程同步仅 HTTPS，失败不得改变本地状态。
5. **恢复**：检测配置外部变化、缺失指针、孤儿托管文件和损坏/不可读状态；提供明确状态和可重试操作。
6. **安全边界**：前端只能通过白名单 Tauri 命令操作；路径必须限制在默认 Codex home 或应用状态目录；不信任 Markdown 内容。

## 技术落点

- Core：沿用 `crates/claude-codex-pro-core/src/system_prompt.rs` 的 `SystemPromptStore`，必要时补齐目标包字段映射。
- Tauri：沿用 `commands.rs` 的 list/save/import/delete/enable/disable/sync 命令及注册表。
- 前端：扩展 `src/components/SystemPromptScreen.tsx`、类型、服务和路由，不引入 Electron API。
- 目标包：仅在取得可构建源码后，按相同数据契约实现其页面；现有 `_extracted` 目录只作为行为证据。
- Leila 部署：`crates/claude-codex-pro-core/src/leila_deploy.rs`，由 Tauri 白名单命令调用。
- CCP 只打包 `gpt5.5-unrestricted.md`、`ac/**`、`leila-identity/**` 和 SHA-256 manifest，固定资源目录为 `apps/claude-codex-pro-manager/src-tauri/resources/leila/assets/`。
- Release 从 Tauri `resource_dir` 读取，开发环境回退到仓库资源目录，不依赖 F 盘。
- 不把约 469 MB 的 `python-wheels` 纳入 CCP 安装包。

## Leila 部署面板

部署面板位于当前状态/启用方式面板下方、分类筛选上方，展示资源版本、平台/架构、Python 版本和位数、模块状态、目标目录、部署/校验/外部修改/回滚状态、最近部署时间、结果和资源 SHA-256。

操作包括“检测环境”“选择 Codex 目录”“部署 Leila”“回滚最近一次”“查看部署日志”。部署日志在面板底部以固定高度区域展开并纵向滚动，新日志追加后自动滚动到末尾；部署或回滚开始时自动展开，用户可手动收起。目标目录必须包含 `config.toml`。仅支持 Windows x64、macOS x64 和 macOS arm64。部署确认必须列出将修改的 `config.toml`、`gpt5.5-unrestricted.md`、`skills/leila-identity` 和 `skills/ac`，并说明 Python 依赖安装需要网络。

## 后端契约

- `inspect_leila_status`：只读检测环境、资源、目标目录和最近一次部署 manifest。
- `choose_leila_codex_target`：选择并校验包含 `config.toml` 的目录。
- `deploy_leila`：检测 Python，按版本映射安装模块，然后备份、暂存替换资源、结构化修改 TOML、校验 SHA-256 并写 manifest。
- `rollback_leila`：只回滚最近一次尚未回滚的成功 CCP 部署，并保留备份历史。

Python 3.8/3.9 使用 `androguard==4.0.1`，3.10-3.14 使用 `androguard==4.1.4`；固定依赖为 `pefile==2024.8.26`、`lief==0.17.6`、`capstone==5.0.9`、`pyelftools==0.32`、`xdis==6.3.0`、`frida==17.16.4`。pip 参数必须包含 `--disable-pip-version-check --only-binary=:all:`。

每次部署在 `.codex/leila-backups/<timestamp>-<operation-id>/` 创建 `manifest.json`、`config.toml.before` 和部署前存在资源的 `.before` 备份。Python 安装失败不得写 `.codex`；资源阶段失败必须恢复本次配置和资源修改。

## 实施阶段

1. 固化数据契约与迁移矩阵，验证：现有 Core 单测全通过。
2. 完成本项目页面能力映射，验证：类型检查、Vite 构建、页面手动检查。
3. 打包 Leila 资源并完成状态、部署、校验和回滚接口，验证：Core 单测和临时 `.codex` 演练。
4. 在系统提示词页接入部署面板，验证：前端类型检查、Vite 构建和页面人工检查。

## 前置条件

支持 Windows x64、macOS x64、macOS arm64 和 Python 3.8-3.14。Leila 资源离线随 CCP 提供，Python 模块安装需要可用网络和动态 pip 源。本任务不生成或重建 Leila EXE。
