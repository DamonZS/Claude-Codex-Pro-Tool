# 供应商目标分流与真实模型可用性修复

## 背景

供应商页同时管理 Codex、Claude 与 Claude Desktop。当前供应商卡片的“使用”按钮无论位于哪个目标标签，都调用 Codex 的供应商切换命令；Claude Desktop 本地模型代理也始终读取 Codex 的 `activeRelayId`。这会造成 Claude/Claude Desktop 与 Codex 串用供应商。

现场检查还确认：管理工具保存的 Claude Desktop 供应商包含 API Key 和模型信息，但写入 Claude Desktop `configLibrary` 的当前 Profile 中 `inferenceGatewayApiKey` 为空。Claude 因此提示“提供商设置需要修复”，并隐藏模型选择入口。代理的 `/v1/models` 已能返回模型，但无效 Profile 使模型目录无法在 Claude 界面生效。

## 目标

- Codex、Claude、Claude Desktop 分别记录当前供应商；切换一个目标不得修改另外两个目标的当前供应商。
- 供应商卡片根据当前标签调用对应目标的切换流程，不得再把 Claude/Claude Desktop 写入 Codex `config.toml`。
- Claude 配置采用新增/合并式写入：只更新所选供应商需要的 `env` 字段，保留现有 `~/.claude/settings.json` 其他字段并创建备份。
- Claude Desktop 写入有效第三方 Profile：本地代理地址、非空凭据和非空模型目录必须同时存在。
- Claude Desktop 本地代理使用 Claude Desktop 当前供应商的上游 URL、API Key 与鉴权配置，不再使用 Codex 当前供应商；映射开启且 `routeId` 精确命中时，真实对话发送保存的 `requestModel`，否则保留入站模型。
- API Key 可以从供应商的直接字段、`authContents`、`configContents.env` 等兼容格式中解析，但不得写入日志或用户提示。
- 切换成功提示必须明确目标；任何配置写入或校验失败都不得显示成功。

## 非目标

- 不删除或重置现有 Codex、Claude、Claude Desktop 配置。
- 不修改 Claude 官方程序文件或汉化资源。
- 不重做供应商页整体视觉布局。
- 不在日志、测试快照或文档中记录真实 API Key。
- 不改变既有 Codex Profile 写入格式和回滚机制。

## 用户视角工作流

1. 用户在供应商页选择 Codex、Claude 或 Claude Desktop 标签。
2. 当前标签只展示对应目标的供应商。
3. 用户点击某个供应商的“使用”。
4. 管理工具校验该供应商的 URL、Key 和必要模型信息，并显示目标明确的处理中提示。
5. Codex 写入 Codex 配置；Claude 合并写入 Claude Code 设置；Claude Desktop 启动本地代理并新增/更新 CCP Profile。
6. 成功后仅当前目标的卡片显示“使用中”。
7. Claude Desktop 完全重启后不再出现“提供商设置需要修复”，并能看到模型选择并完成真实请求。

## 功能要求

### 目标专属当前供应商

- 保留 `activeRelayId` 作为 Codex 当前供应商，兼容已有设置。
- 新增 `activeClaudeRelayId`，用于 Claude。
- 新增 `activeClaudeDesktopRelayId`，用于 Claude Desktop。
- 读取旧设置时新字段缺失不得报错；对应目标的活动 ID 为空时，仅当该目标恰好存在一个可用供应商才回退并选择它，存在零个或多个候选时返回“未配置供应商”（与 `spec/claude-desktop-proxy-upstream-models.md` 保持一致）；任何回退都不得修改 Codex 当前供应商。活动 ID 非空时只允许精确匹配对应目标 Profile，ID 失效时返回“未配置供应商”，不得静默回退到旧 Profile。
- 删除或重命名供应商时，只修正引用该供应商的目标字段。

### API Key 与 URL 解析

- API Key 解析至少支持：
  - Profile 的 `apiKey` / Rust `api_key`
  - `OPENAI_API_KEY`
  - `ANTHROPIC_AUTH_TOKEN`
  - `ANTHROPIC_API_KEY`
  - `api_key` / `apiKey`
  - `authContents` 顶层 JSON
  - `configContents` 顶层或 `env` 嵌套 JSON
- 多个凭据来源同时存在时必须使用目标感知的稳定优先级：用户当前显式输入的 `apiKey` 最高；Claude / Claude Desktop 优先使用 `configContents.env` 的当前凭据，再回退到 `authContents`；Codex 保持 `authContents` 优先于 `configContents` 的既有语义。
- 保存或重新生成 Claude / Claude Desktop 配置时，必须用最终解析出的当前凭据同步 `configContents.env` 与 `authContents`，避免旧凭据在后续加载时再次覆盖当前配置。
- 用户点击“获取模型”“保存”或“保存并使用”时，本次操作必须直接使用编辑框当前的 `apiKey`；不得从保存前的 `configContents`、`authContents` 或旧 Profile 回填覆盖。
- 用户修改或清空 API Key 后，当前编辑值是本次编辑会话的唯一凭据来源。空值表示显式清空：获取模型不得回退旧凭据，保存时必须移除 `configContents`、`authContents` 中全部旧凭据别名，重新打开后不得恢复旧 Key。
- CCSwitch 导入 Profile 可保留未知 JSON 字段，但保存时必须删除旧凭据别名并把当前编辑值写入唯一认证字段；无效 JSON 必须回退为当前表单生成的配置，不能继续携带旧 Key。
- Codex 导入 Profile 保存时必须同时清理 TOML 中的 `experimental_bearer_token`、静态 `Authorization` / `Proxy-Authorization` Header 及其他认证别名；当前编辑 Key 只写入 `authContents.OPENAI_API_KEY`。无法可靠清理的损坏 TOML 必须按当前表单重新生成，不能继续保留旧凭据。
- URL 优先使用供应商的真实上游 URL，不得把 `127.0.0.1` 的 Claude Desktop 本地代理地址再次当作上游。
- 缺少 URL 或 Key 时切换失败，并明确指出缺失字段；不得写入不完整 Profile。

### Claude 新增式写入

- 默认目标文件为 `~/.claude/settings.json`。
- 读取已有 JSON 对象并保留顶层未知字段、permissions、hooks、plugins、MCP 等现有内容。
- 只合并所选供应商 `configContents.env` 中的环境变量，并确保 URL、认证字段和模型字段来自当前供应商。
- 写入前创建时间戳备份；写入失败时原文件保持不变。

### Claude Desktop Profile

- `deploymentMode` 写为 `3p`，保留已有其他 Profile 和 `_meta.json` 条目。
- 当前 CCP Profile 必须包含：
  - `inferenceProvider = gateway`
  - 有效本地 `inferenceGatewayBaseUrl`
  - `inferenceGatewayAuthScheme = bearer`
  - 非空 `inferenceGatewayApiKey`
  - 至少一个 `inferenceModels` 条目
- 模型菜单使用 Claude 安全模型 ID；实际请求模型由所选供应商模型映射决定。
- Claude / Claude Desktop 新建或缺省映射固定显示四个角色，默认值为：Sonnet → `claude-opus-4-6`、Opus → `claude-opus-4-8`、Fable → `claude-Fable-5`、Haiku → `claude-haiku-4-5`，四项默认均声明支持 1M。用户明确填写的实际请求模型必须原样保存，并在映射开启且路由精确命中时用于真实对话。已知的文本显式值与 JSON 安全缺省值分裂时修复 JSON，其他自定义 JSON 映射保持原值。
- Profile 中明确的 `claudeDesktopMode=proxy|direct` 是当前路由模式的事实来源；`configContents.meta.claudeDesktopMode`（含旧 snake_case 字段）只在 Profile 模式缺省时用于兼容迁移。归一化必须同步 `claudeDesktopMode`、`routeEnabled`、`routeMode` 并把最终模式写回规范的 `meta.claudeDesktopMode`；保留其他未知 JSON 字段，重复归一化结果稳定。
- Profile 写入和目标专属当前供应商状态要么同时成功，要么回滚设置状态并返回失败。

### 本地代理

- 管理工具每次启动都必须在后台恢复 Claude Desktop 本地代理；不能要求用户再次点击“使用”或“启动 Claude”后 57331 才开始监听。
- `/claude-desktop/v1/models` 使用 `activeClaudeDesktopRelayId` 对应 Profile 生成模型目录。
- `/claude-desktop/v1/messages` 使用同一 Profile 的上游 URL、Key、User-Agent 和模型映射。
- Claude Desktop 会先对配置的 Gateway 根地址发起 `HEAD /claude-desktop` 健康检查；当前目标的 URL、凭据和模型目录完整时必须返回 2xx，响应与日志不得包含凭据。
- Claude Desktop 的 `POST /claude-desktop/v1/messages/count_tokens?beta=true` 必须返回 Anthropic 兼容的 `{ "input_tokens": number }`，不得落入未知路径 404。
- 当上游未实现 `count_tokens`（例如真实上游返回 404）时，本地代理使用确定性的脱敏本地估算，不因该可选上游能力缺失把整个 Gateway 判定为损坏。
- Gateway 根地址健康检查与 `count_tokens` 都必须写入仅含 method、path、status 的诊断日志，不记录请求正文、Authorization 或 API Key。
- 切换 Claude Desktop 供应商后，Codex `activeRelayId` 必须保持不变。

## UI / 交互要求

- “测试连接”只检查当前表单 Base URL 的网络可达性：只允许向该 URL 发送一次不带认证的 `GET`，不得获取模型、选择模型或请求 `/messages`、`/responses`、`/chat/completions`，也不得发送 API Key、自定义鉴权 Header 或 Body。收到任意 HTTP 状态均表示地址可达，只有 DNS、连接、TLS、超时等传输错误表示连接失败。
- “使用中”状态按当前标签的目标专属 ID 判断。
- 点击 Claude 或 Claude Desktop 供应商不得出现“切换 Codex 供应商”或 `config.toml` 错误提示。
- 点击后立即显示“正在切换 {目标} 供应商”的反馈；完成后显示成功或具体失败原因。
- 本次不新增伪模型下拉框；模型选择必须来自 Claude Desktop 有效 Profile 和模型目录。
- 供应商编辑页“实际请求模型”下拉在 Windows 深色主题中必须使用深色弹层和高对比文字；获取到的模型选项不能因白底浅字而不可读。
- “实际请求模型”不得继续使用会越过 Tauri 窗口边界的系统原生下拉弹层；自定义列表必须约束在当前窗口可视区域内，空间不足时向上展开，并提供有限高度的列表内滚动。
- 成功执行“获取模型”后，下拉可选项必须严格来自该次接口返回的 `models`；不得把默认 Claude 映射、显示名称、旧请求模型或默认兜底模型追加为伪选项。当前已保存但不在返回列表中的值可以显示为当前配置，但必须标记为不可用且不能混入可选列表。
- “一键设置”必须产生可见结果与反馈：保留仍存在于当前供应商模型列表中的有效映射，优先按 Sonnet / Opus / Fable / Haiku 名称匹配，否则统一回退到列表中存在的默认兜底模型或首个真实模型；没有可用模型时提示用户先获取模型，不得静默无效。
- 路由启停只保留供应商列表页外部的目标级总开关：Codex、Claude 与 Claude Desktop 各自独立。切换任一目标只更新该目标的 Profile，不得改写另外两个目标的 `routeEnabled`、`claudeDesktopMode` 或 `routeMode`。编辑表单不得再显示“是否开启路由”开关或可编辑的 `Codex/Claude Desktop Direct/Proxy` 路由文本框；新建和保存供应商时保留当前 Profile 的目标级开关状态。
- API 格式可以提示当前格式需要路由，但不得在编辑表单中自动创建第二套可见路由状态；总开关关闭时应提示返回供应商列表开启对应路由。
- 所有胶囊开关的圆形滑块必须使用几何中心定位，关闭与开启状态都保持垂直居中，不得依赖会受按钮行高影响的基线或块级自动外边距。
- OpenAI 编辑表单的“上游格式”和“模型”必须顶部对齐，两个控件的标签、输入框顶边及高度一致；帮助文字只占左列后续空间，不得把右列输入框向下拉伸。
- Claude 与 Claude Desktop 的“需要模型映射”必须可以开启和关闭。显式 `modelMappingEnabled=false` 必须在归一化、保存和配置生成阶段完整保留；关闭时不生成角色环境变量与 `claudeDesktopModelRoutes`，代理保留入站模型；开启时仅按 `modelMappingJson.routeId` 精确命中改写真实请求模型。已有映射内容保留用于再次开启。
- Claude 与 Claude Desktop 编辑页必须按 CC Switch `ClaudeDesktopProviderForm` 的源码行为渲染两套互斥界面：开启模型映射时显示 API 格式与 Sonnet / Opus / Fable / Haiku 映射矩阵；关闭模型映射时隐藏这两部分，改为显示“手动指定 Claude Desktop 模型列表（高级，可选）”。
- 关闭态手动模型列表必须支持获取上游模型、添加空白模型、编辑模型 ID、逐项声明 1M 和删除条目；`modelList` 使用每行一个模型并以可选 `[1M]` 后缀保存。空行不写入，模型顺序保持用户顺序。
- Claude Desktop 本地模型目录必须将手动列表中每一项的 `[1M]` 声明映射为对应安全模型条目的 `supports1m=true`；未声明的条目不得被错误标记为 1M。
- 开关切换只改变当前生效模式，不得清空另一模式的数据：关闭后保留 `modelMappingJson` / `modelMapping`，重新开启时恢复原映射；开启后同样保留手动 `modelList`，再次关闭时恢复原列表。
- “获取模型”不得自动开启模型映射。关闭态获取成功后应把真实接口模型合并进手动列表并给出反馈，已有条目的 1M 声明不得丢失。
- `/v1/models` 返回 HTTP 2xx 但模型数组为空时必须保留为空结果，不混入映射、固定模型或历史目录；诊断尝试记录为 `empty_payload`，不得记录为已接受有效目录。
- `/v1/models` 模型解析必须支持根数组、`data` / `models` / `items` / `result` / `results` / `payload` / `response` 等多层包装、模型对象映射、JSON 字符串包装，以及 `id` / `model` / `name` / `model_id` / `modelId` / `slug` / `value` 模型标识字段。
- 模型目录解析结果必须区分“有效模型”“明确空数组”“未知响应结构”和“HTTP 2xx 业务错误”。只有明确识别到的空数组记录 `empty_payload`；未知结构记录 `unknown_payload_shape` 并返回明确错误；包含 `error` / `errors` 或显式失败状态的 2xx 响应记录 `business_error`，不得伪装成空目录成功。
- 模型发现诊断只允许记录顶层 JSON 类型、顶层字段名、命中的集合路径、集合长度、首项类型和首项字段名，不记录响应字段值、API Key 或完整响应正文。凭据来源必须描述本次命令收到的 Profile，不得根据持久化标志猜测为旧 Profile。
- Claude Desktop 模型目录必须感知当前模式：映射开启时只从 `modelMappingJson` 生成安全路由 ID、显示名称与 1M 声明，并在真实对话中把精确命中的路由 ID 改写为实际请求模型；映射关闭时以手动列表中的 Claude 安全模型 ID 作为目录 ID，并原样发送入站模型。
- 关闭态手动模型 ID 必须符合 CC Switch 的 Claude Desktop 安全格式：`claude-` 或 `anthropic/claude-` 后接 `sonnet-`、`opus-`、`haiku-`、`fable-` 之一及非空版本；不符合时保存失败并提示开启模型映射。
- 获取模型请求必须绑定发起时的供应商与映射状态。用户在请求完成前切换供应商、切换模式、修改地址/Key 或手动列表时，过期响应必须丢弃，不得污染当前编辑项。
- 删除所有手动模型后允许保存为空，归一化不得用旧默认模型把已删除条目重新补回。
- 配置 JSON 在 API Key 隐藏状态下必须脱敏且只读；只有用户显式点击眼睛显示 Key 后才允许查看和编辑完整配置。
- 供应商编辑页顶部只显示 OpenAI 与 Anthropic 两个官方预设，不显示 DeepSeek、Kimi、Qwen、SiliconFlow、OpenRouter 等预设卡片；其他预设数据可保留供非编辑页入口复用。
- 供应商编辑页不显示“供应商 ID”输入框。内部稳定 ID 仍须保留，用于切换、排序与配置引用；新建供应商可根据名称生成不冲突的内部 ID，编辑已有供应商名称不得改变原 ID。

## 技术约束

- 复用现有 `SettingsStore`、Claude Desktop 本地代理、Profile 新增式写入和日志框架。
- 不引入新的前端或 Rust 依赖。
- 设置迁移必须无损兼容旧 `settings.json`。
- 所有日志只记录供应商 ID、目标、模型数量和状态，不记录 Key。

## 交付范围

- 设置模型与兼容迁移。
- 目标专属切换命令及前端动作。
- Claude 新增式配置写入。
- Claude Desktop Profile、模型目录和代理目标修复。
- API Key 兼容解析。
- Rust/前端回归测试、真实构建与脱敏现场验证。
