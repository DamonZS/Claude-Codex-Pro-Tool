# Leila Codex Offline 系统提示词页面合并验收

对应规格：`spec/leila-offline-system-prompt-merge.md`

## 必须通过

- 系统提示词可从一级导航进入，页面使用本项目统一背景和布局。
- 内置和自定义提示词均可列表、预览、编辑（自定义）、导入、删除、启用和停用。
- preserve/replace 两种模式均保留原配置，可检测外部修改并阻止覆盖；停用可恢复原指针。
- 配置和托管 Markdown 使用原子写入；每次指针变化生成备份；失败时本地状态不丢失。
- 导入限制为 UTF-8 `.md` 且不超过 1 MiB；URL 同步拒绝非 HTTPS 和非 Markdown 响应。
- 缺失指针、孤儿文件、恢复文件、损坏 JSON 和 Windows 权限错误均有可见状态与可重试路径。
- 不改变供应商、模型、注入和 Claude 配置。
- 显式目标目录部署 `gpt5.5-unrestricted.md`、`skills/leila-identity`、`skills/ac`，部署后哈希匹配，旧文件进入 `leila-backups`。
- 缺少 `config.toml`、资源或哈希不匹配时部署失败并恢复配置与已替换文件。
- 页面加载只检测状态，不写入 `.codex`；部署和回滚只由用户点击触发。
- Leila 面板位于当前状态面板下方、分类筛选上方，并提供检测、选择目录、部署、回滚和日志入口。
- 运行状态在面板内固定高度显示并始终展开、可纵向滚动；追加日志后视口自动停留在末尾。
- Windows x64、macOS x64 和 macOS arm64 均可部署；Python 3.8/3.9 与 3.10-3.14 分别选择指定 androguard 版本。
- Python 模块通过隐藏 PowerShell 使用动态 pip 源安装，命令包含 `--disable-pip-version-check` 和 `--only-binary=:all:`；失败时显示 pip 输出摘要且不开始资源写入。
- Release 使用 Tauri `resource_dir`，开发环境可回退仓库资源目录；运行时不依赖 F 盘，且安装包不包含 `python-wheels`。
- 每次资源写入前创建唯一备份目录和 manifest；最近一次部署可回滚，原来不存在的文件会删除，原来存在的文件会恢复。
- 重复部署不会重复替换已匹配资源；不修改 `hooks/instruction-inject.sh`。

## 验证证据

```powershell
cargo test -p claude-codex-pro-core system_prompt -- --nocapture
cargo test -p claude-codex-pro-core leila_deploy -- --nocapture
cargo check -p claude-codex-pro-manager
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo fmt --check
```

部署验证另需：临时目标目录中的 `config.toml`、三类资源、SHA-256、操作 manifest 和回滚后文件状态证据。不得为验收写入真实用户 `.codex`。

## 当前限制

本任务不验收 Leila EXE 的源码级重建或新安装包。真实 pip 下载结果受当前动态源和网络可用性影响。
