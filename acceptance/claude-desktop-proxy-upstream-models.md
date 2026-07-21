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
   - 通过标准：`/v1/models` 返回非空目录时只返回该目录，不请求价格接口，也不混入 `modelList`、`modelMapping`、`modelMappingJson` 或本地默认模型。

7. 标准空目录保持真实失败
   - 通过标准：`/v1/models` 返回 HTTP 200 空数组后明确失败，服务端只收到标准模型目录请求，不收到价格接口请求。

8. 标准失败不被兼容目录掩盖
   - 通过标准：标准模型接口返回鉴权错误、429、5xx 或网络错误时保持原失败，不请求价格接口，也不回退到本地默认模型或已有映射。
   - 通过标准：所有真实对话和连接测试都不调用模型目录；只有用户主动“获取模型”读取标准目录，HTTP 2xx 空目录时保持真实失败。

9. 凭据不泄漏
   - 通过标准：成功结果、失败文本和测试输出不包含 API Key、Bearer token、URL query 或 fragment。

10. 推理可用性独立
   - 通过标准：规格和实现不因目录中存在某模型就宣称当前 Key 可调用；上游 503 通道错误不触发自动模型映射修改。

11. 真实对话按显式映射发送
   - 通过标准：每个 Claude Desktop Messages 入站请求不请求 `/v1/models`，只向当前活动 Profile 的上游发送一次 `/v1/messages`；模型目录异常不能阻断真实对话。
   - 通过标准：`modelMappingEnabled=true` 且 `modelMappingJson.routeId` 精确命中时，Opus、Sonnet、Haiku、Fable 与 Subagent 均发送对应 `requestModel`；Body 覆盖不能撤销该结果。
   - 通过标准：映射关闭或 `routeId` 未命中时逐字保留入站模型；不从 `modelMapping` 文本、`modelList`、默认角色或历史发现结果执行关键词、角色或首项回退。
   - 通过标准：消息 POST 使用当前活动 Profile 的解析后 Key、上游 Base URL 和 Bearer/`x-api-key` 语义，不回退到全局 Key、环境变量 Key或其他 Profile。

12. 探针形状不触发特殊模型选择
   - 通过标准：仅包含 `model`、`max_tokens=1` 和单条 `.` 消息的请求仍按第 11 项处理，不请求模型目录、不按 Opus、Sonnet、Haiku 或 Fable 家族猜测模型。
   - 通过标准：映射开启且精确命中时发送保存的 `requestModel`，映射关闭或未命中时原样发送入站模型。

13. 映射配置数据保持兼容
   - 通过标准：`角色 (routeId): displayName -> requestModel [1M]` 可无损解析；空 JSON 可从有效文本重建；文本明确选择 `claude-opus-4-7` 而 JSON 错写 `claude-haiku-4-5` 的已知分裂状态被修复为 `claude-opus-4-7`；其他自定义 JSON 值不被文本覆盖；修复后 `modelMappingJson` 与 `modelMapping` 一致，相同设置重复归一化结果稳定。
   - 通过标准：当 `configContents.meta.claudeDesktopModelRoutes` 与 `modelMappingJson` 分裂时，相同 `routeId` 使用配置路由中已保存的 `model`、`labelOverride` 与 `supports1m`，缺失路由被补齐，JSON 独有的自定义路由被保留；无效配置不覆盖映射。修复后的映射按第 11 项应用于真实对话。
   - 通过标准：`configContents.meta.claudeDesktopMode=proxy` 时同步 `claudeDesktopMode=proxy`、`routeEnabled=true` 和 `routeMode=Claude Desktop Proxy`；值为 `direct` 时同步为 direct、false 和 `Claude Desktop Direct`，重复归一化结果稳定。

14. 路由诊断脱敏
   - 通过标准：Claude Desktop Messages 上游响应后记录 Profile ID、原模型、实际上游模型和 HTTP 状态，记录中不包含 Key、URL、请求正文或响应正文。

15. 活动供应商选择严格且兼容唯一旧配置
   - 通过标准：Claude / Claude Desktop 在活动 ID 有效时精确选择对应 Profile；ID 无效时不回退；ID 为空且存在多个候选时返回“未配置供应商”；ID 为空且只有一个候选时仍可选择该 Profile。

16. Codex 模型原样透传且不额外双发
   - 通过标准：回归测试验证 `gpt-5.6-sol` 经 Responses -> Chat Completions 转换后保持不变；代理实现对每个正常入站请求只执行一次上游推理 POST，模型发现流程不调用推理端点。

17. Anthropic 连接测试、模型发现与真实消息相互独立
   - 通过标准：供应商“测试连接”只向当前 Profile 的 Base URL 发送一次 GET，包含 `Accept: */*` 与 `Accept-Encoding: identity`，不请求 `/v1/models` 或 `/v1/messages`，不发送 Key、模型或正文。
   - 通过标准：任意 HTTP 状态均返回“Base URL 可访问”并保留实际状态；监听端口不存在等传输错误返回失败，且不进行第二次请求。
   - 通过标准：用户主动“获取模型”仍使用当前 Profile 和当前 Key 请求 `/v1/models`；真实 Claude Desktop 消息仍只向 `/v1/messages` 单次 POST，并按当前 Profile 选择 Bearer 或 `x-api-key` 与 `anthropic-version`。

18. 最新 Key 全链路一致
   - 通过标准：Profile 原有旧 Key，编辑器传入新 Key 后执行保存再加载，`configContents` 与 `authContents` 中旧凭据别名均被新 Key 替换。
   - 通过标准：重新打开编辑器、主动获取模型、应用 Claude 配置和 Claude Desktop 消息代理均使用该最新 Key，不回退到旧容器、全局 Key、环境变量或其他 Profile。

19. Claude Beta 请求语义完整透传
   - 通过标准：集成测试验证入站 `?beta=true`、`anthropic-version` 与 `anthropic-beta` 到达单次 Messages 上游请求；客户端 Authorization / `x-api-key` 不会覆盖管理工具解析出的上游凭据。

20. 正常 SSE 在任意网络拆包下保持完整
   - 通过标准：单元测试将正常多块 SSE 按事件中间、CRLF 边界和 JSON 字节中间拆分，输出事件字节与输入一致，生命周期异常计数为零。

21. 异常内容块生命周期不会导致客户端崩溃
   - 通过标准：单元测试覆盖缺少 start 的文本/思考 delta、无法推断身份的 `input_json_delta`、孤立 stop 和 `message_stop` 前未关闭块；输出中不存在无对应 start 的 delta/stop，不伪造工具 ID/name，并生成不包含正文和工具参数的有界诊断摘要。

22. HTTP 200 与协议成功分开记录
   - 通过标准：正常完整流记录协议成功；修复过的流记录修复结果；截断或未闭合流记录协议不完整，不能继续沿用普通 `helper.claude_desktop_messages_proxy_stream_ok`。

23. 真实验证通过
   - 通过标准：至少运行并通过：
     - `cargo fmt --check`
     - `cargo test -p claude-codex-pro-core --test model_catalog -- --nocapture`
     - `cargo test -p claude-codex-pro-core --test relay_config -- --nocapture`
     - `cargo test -p claude-codex-pro-core protocol_proxy -- --nocapture`
     - `cargo test -p claude-codex-pro-core --test claude_desktop_provider -- --nocapture`
     - `cargo test -p claude-codex-pro-manager --test windows_subsystem -- --nocapture`
     - `npm --prefix apps/claude-codex-pro-manager run check`
     - `cargo build -p claude-codex-pro-manager`
     - `cargo build --release`

24. 模型目录候选回退与诊断
   - 通过标准：Base URL 为根路径、`/v1`、其他版本段或已知兼容子路径时，候选端点顺序稳定、去重，并返回实际成功的端点。
   - 通过标准：首个候选返回 404/405 时按顺序尝试后续候选；首个候选返回 401/403/429/5xx、网络错误或 200 空目录时不以其他候选掩盖该结果。
   - 通过标准：所有候选请求使用当前 Profile 解析出的同一 Key 与鉴权头；诊断包含脱敏端点、HTTP 状态、解析模型数量和最终结果，不包含 Key、查询参数或响应正文。

## 2026-07-20 补充验收：Claude Desktop 请求覆盖

- 构造带 `headerOverride={"X-Provider":"cc-switch"}` 与 `bodyOverride={"temperature":0.2}` 的 Claude Desktop Profile 时，真实 `/v1/messages` 出站请求必须包含 `X-Provider: cc-switch` 和 `temperature: 0.2`。
- 同一测试中，`Authorization`、`x-api-key`、`Content-Type`、`Host`、`Content-Length` 等受保护 Header 不被覆盖。
- `bodyOverride.stream` 不改变代理流式判定和出站 `stream` 字段。
- 诊断日志只记录覆盖字段名，不记录覆盖值或凭据。

## 2026-07-20 补充验收：Claude Desktop 上游失败脱敏诊断

- 上游 `/v1/messages` 返回非 2xx 时，日志存在 `proxy.claude_desktop_upstream_error_body`，包含 status、content_type、error_type、error_message 摘要，不包含 API Key、Bearer token、完整 URL query、完整请求正文或完整响应正文。
- `proxy.claude_desktop_upstream_route` 可区分本次实际发送的 upstream_path、auth_header_mode、beta_query、has_anthropic_version、has_anthropic_beta、is_stream 和 request_body_fields，用于定位真实报错原因。

## 2026-07-21 补充验收：CC Switch-compatible 模型目录鉴权

- Claude 与 Claude Desktop Profile 无论当前 Key 来源字段是 `ANTHROPIC_AUTH_TOKEN` 还是 `ANTHROPIC_API_KEY`，用户主动“获取模型”都只向 `/v1/models` 发送 `Authorization: Bearer <当前 Key>`。
- 模型目录请求不发送 `x-api-key` 与 `anthropic-version`；该行为不改变真实 `/v1/messages` 的鉴权语义。
