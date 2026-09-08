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

## 验证证据

```powershell
cargo test -p claude-codex-pro-core system_prompt -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo fmt --check
```

目标包阶段另需：离线启动日志、功能录屏或截图、重打包产物绝对路径、干净目录回滚记录。

## 当前限制

在目标包只有 Electron 打包产物时，只能验收本项目页面和 Core 行为；不能验收目标 EXE 的源码级重建或新安装包。
