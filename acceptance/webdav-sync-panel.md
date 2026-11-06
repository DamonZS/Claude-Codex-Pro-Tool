# WebDAV 配置备份面板 — 通过标准

对应规格：[`spec/webdav-sync-panel.md`](../spec/webdav-sync-panel.md)。勾选表示有证据；真机点击项由 Lead 在合并前执行。

- [x] **F1 类型检查**：`npm --prefix apps/claude-codex-pro-manager run check` 退出码 0。
- [x] **F2 生产构建**：`npm --prefix apps/claude-codex-pro-manager run vite:build` 退出码 0，`dist/assets` 生成新的 `index-*.js` / `index-*.css`。
- [x] **F3 挂载位置**：`SettingsScreen` 的 `appPaths` 为 `<>` 片段：`MaintenanceToolsPanel` 之后是 `WebdavSyncPanel`，原有顺序不变。
- [x] **F4 三处登记**：`loadWebdavSync` / `saveWebdavSync` / `uploadWebdavBackup` / `downloadWebdavBackup` 同时存在于 `createActionsShape()`、`actionsRef.current` 与 `useMemo<AppActions>`，无 TS2741。
- [ ] **F5 挂载回填**（真机，Lead）：进入设置页触发 `load_webdav_sync`，三个输入框回填地址、用户名与密码（密码掩码显示）。
- [ ] **F6 保存配置**（真机，Lead）：点「保存配置」发出 `save_webdav_sync`，参数 `{ request: { baseUrl, username, password } }`；成功后输入框以后端返回值回填。
- [ ] **F7 立即上传**（真机，Lead）：点「立即上传备份」发出 `upload_webdav_backup`（无参），成功后显示本次 `remoteUrl` 与 `lastFile`。
- [ ] **F8 无备份时拉取拦截**（真机，Lead）：`lastFile` 为空时点「从远端拉取」不发命令，弹出「本机还没有上传过备份，请先点『立即上传备份』」。
- [ ] **F9 拉取备份**（真机，Lead）：`lastFile` 存在时点「从远端拉取」发出 `download_webdav_backup`，参数 `{ request: { fileName } }`。
- [ ] **F10 失败与禁用**（真机，Lead）：操作中按钮禁用；失败时保留表单值，仅右下角失败提示。
- [ ] **F11 密码掩码**（代码评审）：密码输入 `type="password"`，不写入任何日志或提示文案。

## 非通过条件

- 面板插在 `MaintenanceToolsPanel` 之前，或破坏 `appPaths` 原有内容。
- 命令参数结构错误（如 `download` 未传 `{ request: … }`）。
- 失败时清空表单或把失败载荷当成功数据渲染。
- 新增 CSS、第三方依赖，或改动 Rust 文件 / 预览 mock。