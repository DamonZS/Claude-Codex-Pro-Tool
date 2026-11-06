# 供应商分享链接 — 通过标准

1. **编码往返**：中文名称的供应商（含 `baseUrl`/`upstreamBaseUrl`/`model`/`targetApp`）编码后再解码，五个字段与输入一致（`src/lib/supplier-share.test.ts` 第 1 例）。
2. **不含 API Key**：编码结果解码后的键集合恰为 `baseUrl/model/name/targetApp/upstreamBaseUrl`，且序列化文本不含 `sk-`（第 2 例）。
3. **拒绝异类输入**：`ccswitch://provider?id=abc`、非法 base64、缺 `baseUrl` 的 JSON 三种输入都返回 `null`（第 3 例）。
4. **测试与类型**：`npx vitest run src/lib/supplier-share.test.ts` 3 passed；`npm run check` 退出码 0。
5. **构建**：`npm run vite:build` 退出码 0，产物包含 `ccp://supplier?d=`。
6. **界面落点**：供应商卡片操作区出现分享图标（title「复制分享链接（不含 API Key）」）；「从第三方导入」弹层出现「分享链接」项，展开后有链接输入框与「导入」按钮。
7. **导入不越权**：导入后只进入供应商编辑器草稿，未保存前不写 `ccp.db`、不写 `~/.codex`。

## 非通过条件

- 链接中出现 API Key 或 config/auth 全文。
- 导入直接落库或直接写入 Codex / Claude 配置文件。