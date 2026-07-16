# 验收标准：Claude Desktop 代理上游与模型发现修复

验证对象：`spec/claude-desktop-proxy-upstream-models.md`

## 验收项

1. 规格与验收文档存在
   - 通过标准：本文件和 `spec/claude-desktop-proxy-upstream-models.md` 存在。

2. 默认模型列表为四项
   - 通过标准：测试验证空模型列表时，Claude Desktop 模型发现返回 `claude-fable-5`、`claude-haiku-4-5`、`claude-opus-4-8`、`claude-sonnet-4-6`。

3. Profile 写入包含默认四模型
   - 通过标准：测试验证 provider/dev-mode profile 在模型列表为空时仍写入四个 `inferenceModels`。

4. 上游 Base URL 不再为空
   - 通过标准：测试验证 active relay 上游为空时，Claude Desktop 代理解析到 `https://api.toporeduce.cn`，不会因 Base URL 为空直接 502。

5. 获取模型使用解析后 Key 与正确字段鉴权
   - 通过标准：集成测试构造 `api_key` 为空、Key 只存在于 `authContents` 或 `configContents` 的 profile；`ANTHROPIC_AUTH_TOKEN` / `OPENAI_API_KEY` 的 `/v1/models` 单次请求只收到 Bearer，`ANTHROPIC_API_KEY` 单次请求只收到 `x-api-key` 与 `anthropic-version`，Codex/OpenAI 格式保持 Bearer。

6. 标准模型目录优先且与映射解耦
   - 通过标准：`/v1/models` 返回非空目录时只返回该目录，不请求 `/api/pricing`，也不混入 `modelList`、`modelMapping`、`modelMappingJson` 或本地默认模型。

7. 标准空目录保持真实失败
   - 通过标准：`/v1/models` 返回 HTTP 200 空数组后明确失败，且服务端只收到 `/v1/models` 请求，不收到 `/api/pricing` 请求。

8. 失败不被公开目录掩盖
   - 通过标准：标准模型接口返回鉴权、空目录或服务错误时保持失败，不请求公开价格目录，也不回退到本地默认模型或已有映射。

9. 凭据不泄漏
   - 通过标准：成功结果、失败文本和测试输出不包含 API Key、Bearer token、URL query 或 fragment。

10. 推理可用性独立
   - 通过标准：规格和实现不因目录中存在某模型就宣称当前 Key 可调用；上游 503 通道错误不触发自动模型映射修改。

11. 活动供应商选择严格且兼容唯一旧配置
   - 通过标准：Claude / Claude Desktop 在活动 ID 有效时精确选择对应 Profile；ID 无效时不回退；ID 为空且存在多个候选时返回“未配置供应商”；ID 为空且只有一个候选时仍可选择该 Profile。

12. Codex 模型原样透传且不额外双发
   - 通过标准：回归测试验证 `gpt-5.6-sol` 经 Responses -> Chat Completions 转换后保持不变；代理实现对每个正常入站请求只执行一次上游推理 POST，模型发现流程不调用推理端点。

13. Claude Desktop 原生消息鉴权兼容
   - 通过标准：回归测试分别验证 `ANTHROPIC_AUTH_TOKEN` 与 `ANTHROPIC_API_KEY`；每次 Claude Desktop Messages 上游 POST 只包含对应的 Bearer 或 `x-api-key`，均包含 `anthropic-version: 2023-06-01`，且没有鉴权重试或重复推理请求。

14. Claude Beta 请求语义完整透传
   - 通过标准：集成测试验证入站 `?beta=true`、`anthropic-version` 与 `anthropic-beta` 到达单次 Messages 上游请求；客户端 Authorization / `x-api-key` 不会覆盖管理工具解析出的上游凭据。

15. 正常 SSE 在任意网络拆包下保持完整
   - 通过标准：单元测试将正常多块 SSE 按事件中间、CRLF 边界和 JSON 字节中间拆分，输出事件字节与输入一致，生命周期异常计数为零。

16. 异常内容块生命周期不会导致客户端崩溃
   - 通过标准：单元测试覆盖缺少 start 的文本/思考 delta、无法推断身份的 `input_json_delta`、孤立 stop 和 `message_stop` 前未关闭块；输出中不存在无对应 start 的 delta/stop，不伪造工具 ID/name，并生成不包含正文和工具参数的有界诊断摘要。

17. HTTP 200 与协议成功分开记录
   - 通过标准：正常完整流记录协议成功；修复过的流记录修复结果；截断或未闭合流记录协议不完整，不能继续沿用普通 `helper.claude_desktop_messages_proxy_stream_ok`。

18. 真实验证通过
   - 通过标准：至少运行并通过：
     - `cargo fmt --check`
     - `cargo test -p claude-codex-pro-core --test model_catalog -- --nocapture`
     - `cargo test -p claude-codex-pro-core protocol_proxy -- --nocapture`
     - `cargo test -p claude-codex-pro-core --test claude_desktop_provider -- --nocapture`
     - `cargo test -p claude-codex-pro-manager --test windows_subsystem -- --nocapture`
     - `npm --prefix apps/claude-codex-pro-manager run check`
     - `cargo build -p claude-codex-pro-manager`
