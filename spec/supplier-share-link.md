# 供应商分享链接（ccp://supplier）

## 背景

cc-switch 用 `ccswitch://` 深链在用户之间传递供应商配置（`src-tauri/src/deeplink/provider.rs` + `deplink.html` 分享页）。CCP 目前只能从 cc-switch / Codex / Claude Desktop 的本地文件**导入**，没有把一条配置**传给别人**的通道。

本次只做"复制一条链接 / 粘一条链接导入"，不注册系统协议、不做分享页。

## 功能要求

- F1：供应商卡片新增「复制分享链接」，点击把 `ccp://supplier?d=<base64url(JSON)>` 写入剪贴板，并提示已复制。
- F2：链接只携带 `name`、`baseUrl`、`upstreamBaseUrl`、`model`、`targetApp` 五个字段；**API Key 永不进入链接**。
- F3：「从第三方导入」弹层新增「分享链接」入口，粘贴链接后点「导入」，解析出的字段填入供应商编辑器草稿（新建一个未保存的供应商），由用户补 Key、确认模型后自行保存。
- F4：链接不是 `ccp://supplier?d=` 前缀、或 base64/JSON 解析失败、或缺少 `baseUrl` 时，给明确错误提示且不改变任何现有配置。
- F5：编解码为纯前端实现（`src/lib/supplier-share.ts`），可被单测覆盖。

## 非目标

- 不注册 `ccp://` 系统协议，不做 URL 唤起。
- 不生成/托管网页分享页（cc-switch 的 `deplink.html` 不在本次范围）。
- 不分享 API Key、configContents、authContents、模型目录等大字段。

## 技术约束

- 编码：UTF-8 JSON → base64url（`-`/`_`，去尾部 `=`），保证中文名称可用。
- `baseUrl` 优先取 `upstreamBaseUrl`，回退 `baseUrl`；`model` 优先取 `testModel`，回退 `model`。
- 导入只写编辑器草稿，不直接落库，不触发任何写文件动作。

## 变更记录

- 2026-11-06：新建。对应 `docs/cc-switch-port-proposal.md` 的 P1-2。