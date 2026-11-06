# WebDAV 配置备份面板

## 背景

CCP 的配置分散在本机多个文件（设置、供应商 Profile、Skill 与上下文等）。用户换机、重装或做灾备时，目前只能手工拷贝目录。后端已实现 WebDAV 备份命令：`load_webdav_sync`、`save_webdav_sync`、`upload_webdav_backup`、`download_webdav_backup`——把配置打包上传到用户自己的 WebDAV 目录，或从远端拉回本机。

本规格只做前端接线：在设置页「应用与路径」区域增加「WebDAV 配置备份」面板，让用户填写地址/账号、保存、上传与拉取。

## 用户视角

用户进入设置页，在既有「应用与路径」维护工具下方看到「WebDAV 配置备份」：三个输入框已回填上次保存的地址和用户名（密码同样回填但以掩码显示）。填写后点「保存配置」写入本机；点「立即上传备份」上传当前配置，成功后显示本次远端地址；点「从远端拉取」下载上次上传的备份。若从未上传过，拉取时提示先上传。

## 功能要求

- F1：`SettingsScreen` 的 `appPaths` 区域在既有 `MaintenanceToolsPanel` 之后渲染 `WebdavSyncPanel`，标题为「WebDAV 配置备份」，不破坏原有布局与顺序。
- F2：面板挂载时调用 `actions.loadWebdavSync()`（命令 `load_webdav_sync`），回填 `baseUrl`、`username`、`password` 三个输入框，并保留 `lastFile` / `remoteUrl` 供展示。
- F3：提供「WebDAV 地址」「用户名」「密码」三个输入；密码输入使用 `type="password"`。输入变化只更新本地草稿，不自动保存。
- F4：「保存配置」调用 `actions.saveWebdavSync(baseUrl, username, password)`（命令 `save_webdav_sync`，参数 `{ request: { baseUrl, username, password } }`），成功后用返回载荷回填输入框（以后端规范化后的值为准）。
- F5：「立即上传备份」调用 `actions.uploadWebdavBackup()`（命令 `upload_webdav_backup`），成功后显示本次上传的 `remoteUrl`，并展示 `lastFile`。
- F6：「从远端拉取」在 `lastFile` 为空时通过 `actions.showNotice` 提示「本机还没有上传过备份，请先点『立即上传备份』」且不发请求；否则调用 `actions.downloadWebdavBackup(lastFile)`（命令 `download_webdav_backup`，参数 `{ request: { fileName } }`）。
- F7：任一操作进行中禁用输入与按钮；操作失败（action 返回 `null`）时保留已有表单值与展示数据，只弹出失败提示。

## 非目标

- 不修改 Rust 命令、WebDAV 客户端实现、备份内容与远端目录结构；不新增远端文件命名规则。
- 不做自动定时上传、冲突合并、多账号管理或凭据加密存储（凭据存储由后端既有设置机制负责）。
- 不新增 CSS/第三方依赖，不改预览 mock。
- 不做「删除远端备份」「浏览远端目录」等额外能力。

## 技术约束

- 仅允许改动：`src/components/WebdavSyncPanel.tsx`（新建）、`src/screens.tsx`、`src/lib/actions.ts`、`src/App.tsx`、`src/types.ts`。
- 命令统一经 `call<CommandResult<WebdavSyncPayload>>(命令名, 参数)`；Rust 把 payload 展开到顶层，故直接读取 `baseUrl` / `username` / `password` / `lastFile` / `remoteUrl`。
- `download_webdav_backup` 的 `request` 是 `Option<DownloadWebdavSyncRequest>`：无文件名时传 `{ request: null }`。
- action 约定：失败时 `notifyResult` 提示后返回 `null`，成功时返回载荷；面板以 `null` 判定失败。
- 只复用 `Panel`、`InfoRow`、`Button` 与既有类名（`stack`、`action-row`、`ops-form-field`）；不新增样式。
- 新增 action 必须在 `createActionsShape()`、`actionsRef.current`、`useMemo<AppActions>` 三处同时登记，否则触发 TS2741。

## 变更记录

- 2026-11-06：新建。对应后端 `settings_commands.rs` 中四条 WebDAV 命令的前端接线。