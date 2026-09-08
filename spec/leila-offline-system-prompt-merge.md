# Leila Codex Offline 系统提示词页面合并方案

## 背景

用户要求将 `F:\迅雷下载\Leila-Codex-Offline-1.0.7-windows-python38-314-AC\Leila-Codex-Offline-1.0.7-windows-python38-314-AC` 中的系统提示词页面与功能合并到本项目的系统提示词页面；目标包当前是 Electron 37.2.6 打包产物，已提取出 `main.js`、`preload.js`、`renderer/app.js`、`renderer/styles.css` 和资源目录，没有可直接编译的源工程。

## 目标

- 在本项目现有 `SystemPromptScreen` 中承载目标包已确认的提示词浏览、启用、编辑、导入、同步和环境状态能力。
- 复用现有 Tauri/Core 的存储、原子写入、备份、外部修改检测和恢复语义。
- 保持本项目统一导航、背景特效、颜色和交互风格；目标包视觉仅作为信息架构参考。
- 保持 `model`、`provider`、注入和供应商配置不变。

## 非目标

- 不把目标包中的 Electron 主进程代码直接复制到 React/Tauri。
- 不迁移目标包的 `ac` 或其他不可信提示词内容作为实现指令。
- 不修改 Claude Desktop 官方文件，不改变供应商路由或默认模型。
- 在没有目标包可构建源码和重打包链前，不声称已生成新的 Leila EXE。

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

## 实施阶段

1. 固化数据契约与迁移矩阵，验证：现有 Core 单测全通过。
2. 完成本项目页面能力映射，验证：类型检查、Vite 构建、页面手动检查。
3. 做目标包资源/行为适配，验证：离线启动、CRUD、preserve/replace、外部修改和恢复流程。
4. 获取源码后重建并打包目标 EXE，验证：干净目录安装和回滚演练。

## 前置条件

第 3、4 阶段需要目标包的可构建源码、依赖锁文件和合法的 Electron 打包脚本；当前仅有打包产物，不能可靠地反编译回可维护工程。
