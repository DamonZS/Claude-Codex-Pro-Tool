# 第三方供应商导入完整性修复验收标准

对应规格：`spec/supplier-third-party-import-completeness.md`

## 验收项

1. 导入范围完整
   - 通过标准：cc-switch `providers` 中 `codex`、`claude`、`claude-desktop` 类型均会被扫描，符合条件的记录会生成供应商 profile。
   - 验证方式：Rust 定向测试覆盖三类 app_type。

2. Codex 原始配置不丢失
   - 通过标准：导入后的 `configContents` 保留原始 TOML，包含原始 provider、插件或额外配置；前端保存时不被默认生成配置覆盖。
   - 验证方式：Rust 测试 + 前端静态检查。

3. API Key 读取完整且脱敏
   - 通过标准：可从 auth、`experimental_bearer_token`、`http_headers.Authorization` 中读取测试密钥；不输出真实密钥。
   - 验证方式：使用假密钥的 Rust 测试。

4. Claude 配置可导入
   - 通过标准：`ANTHROPIC_BASE_URL`、`ANTHROPIC_AUTH_TOKEN`、`ANTHROPIC_MODEL`、`ANTHROPIC_DEFAULT_*_MODEL` 会映射到 profile 字段与模型映射展示信息。
   - 验证方式：Rust 测试断言 base URL、模型和映射。

5. Anthropic 预设可见
   - 通过标准：供应商预设模板中可选择 Anthropic / Claude，应用后填入 Anthropic base URL、Claude 模型、Anthropic Messages API 格式与 Claude Desktop 目标。
   - 验证方式：前端静态测试、类型检查与构建通过。

6. API 格式完整
   - 通过标准：Claude / Claude Desktop 配置区域包含 Anthropic Messages、OpenAI Chat Completions、OpenAI Responses API、Gemini Native generateContent 四种格式，并标明哪些需要路由。
   - 验证方式：前端静态测试、类型检查与构建通过。

7. cc-switch 路由语义可审查
   - 通过标准：UI 可见“是否开启路由”；模型映射包含路由 ID、菜单显示名、实际请求模型、1M 标记；导入的 `[1M]` 后缀会转为 `supports1m`。
   - 验证方式：Rust 测试断言导入结果，前端静态测试断言路由列与开关文案存在。

8. cc-switch 元信息保留
   - 通过标准：从 cc-switch 导入时能保留/派生 `api_format`、`claude_desktop_mode`、`claude_desktop_model_routes` 或 `ANTHROPIC_DEFAULT_*_MODEL` 路由。
   - 验证方式：Rust 测试覆盖 Proxy 路由与 Anthropic Direct。

9. UI 展示来源、路由和 API 格式
   - 通过标准：供应商卡片/编辑页可见导入来源、目标应用、API 格式、是否开启路由、模型映射或模型列表。
   - 验证方式：前端类型检查与构建通过，手动打开页面检查。

10. 不破坏现有功能
    - 通过标准：现有 Codex 供应商切换字段仍存在；老 profile 缺少新字段也能正常渲染。
    - 验证方式：前端 check/build 与 Rust 定向测试通过。

11. 供应商拖拽排序可用
   - 通过标准：供应商卡片不再启用整卡片 HTML5 draggable，只通过左侧拖拽柄的 pointer 事件排序；移动过程基于最新顺序计算落点；松手后保存并给出状态提示。
   - 验证方式：前端静态测试断言 pointer-only 结构、类型检查、前端构建和 manager 构建通过。

## 必需验证命令

- `cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml ccswitch -- --nocapture`
- `cargo test -p claude-codex-pro-core --manifest-path Cargo.toml --test protocol_proxy claude_desktop_model_mapping -- --nocapture`
- `npm --prefix apps/claude-codex-pro-manager run check`
- `npm --prefix apps/claude-codex-pro-manager run vite:build`
- `cargo build -p claude-codex-pro-manager --manifest-path Cargo.toml`

## 非目标

- 不要求本次完成新的供应商分类系统。
- 不要求连接真实第三方服务。
- 不要求展示完整密钥，除非用户在本地 API Key 输入框主动点击眼睛查看。
