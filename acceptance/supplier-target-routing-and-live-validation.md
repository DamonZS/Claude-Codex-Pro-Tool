# 验收标准：供应商目标分流与真实模型可用性修复

验证对象：`spec/supplier-target-routing-and-live-validation.md`

## 自动化验收

1. **旧设置兼容与目标独立**
   - 缺少 `activeClaudeRelayId` / `activeClaudeDesktopRelayId` 的旧设置可以正常加载。
   - 设置可分别保存并加载三个目标的当前供应商 ID。
   - 切换 Claude Desktop 后 `activeRelayId` 不变。

2. **API Key 兼容解析**
   - 测试覆盖 `OPENAI_API_KEY`、`ANTHROPIC_AUTH_TOKEN`、`ANTHROPIC_API_KEY`、`api_key`、`apiKey`。
   - 测试覆盖 `authContents` 和 `configContents.env` 嵌套 JSON。
   - 测试和日志中不出现测试密钥之外的真实凭据。

3. **Claude 新增式写入**
   - 预置包含未知顶层字段、permissions、hooks 的 `settings.json` 后切换供应商，这些字段保持不变。
   - `env` 中目标供应商 URL、认证和模型字段被更新。
   - 写入前产生备份；无效 JSON 或写入失败时不损坏原文件。

4. **Claude Desktop Profile 有效**
   - 缺少 API Key 的供应商被拒绝，不产生不完整 Profile。
   - 有效供应商写入后 `inferenceGatewayApiKey` 非空、`inferenceGatewayBaseUrl` 指向当前本地代理、`inferenceModels` 至少一项。
   - 原有 CC Switch 与其他 Profile 文件和 meta 条目保持不变。
   - 缺省映射的四个角色分别为 Sonnet=`claude-opus-4-6`、Opus=`claude-opus-4-8`、Fable=`claude-Fable-5`、Haiku=`claude-opus-4-7`，且 `supports1m` 全部为 true。

5. **代理使用正确目标**
   - 仅重启管理工具、不重新点击供应商“使用”时，Claude Desktop 本地代理会自动恢复监听。
   - 当 Codex 与 Claude Desktop 选择不同供应商时，模型目录与消息代理读取 Claude Desktop 当前供应商。
   - 模型映射把 Claude 安全模型 ID 转换为当前供应商配置的实际请求模型。
   - `HEAD /claude-desktop` 命中健康检查路由，配置完整时返回 2xx，不再返回未知路径 404。
   - `POST /claude-desktop/v1/messages/count_tokens?beta=true` 返回 JSON 数字字段 `input_tokens`，不再返回未知路径 404。
   - 对更长的可解析会话正文，`input_tokens` 估算值必须单调增加；无效 JSON 返回明确错误而不是伪造成功。

6. **前端目标分流**
   - 供应商卡片不再写死调用 `switchCodexRelayProfile`。
   - Codex、Claude、Claude Desktop 标签分别按 `activeRelayId`、`activeClaudeRelayId`、`activeClaudeDesktopRelayId` 显示“使用中”。
   - Claude/Claude Desktop 切换提示中不出现 Codex。
   - 获取模型后展开“实际请求模型”下拉，所有选项在深色主题下文字与背景对比清晰，不出现白底浅字。
   - 模型列表使用窗口内自定义弹层，具备固定定位、可视区边界计算、最大高度和纵向滚动；列表不会越过管理工具窗口。
   - 获取模型成功后可选项只包含该次 `models` 返回值；默认 Claude 映射和旧映射不得混入选项。旧值不在返回列表中时仅显示为“当前配置不可用”。
   - 点击“一键设置”后四个角色的实际请求模型均属于当前供应商模型列表，并显示成功反馈；无模型时显示“请先获取模型”反馈。
   - 编辑表单中不存在“是否开启路由”和“路由”文本框，列表页仍保留目标级“开启路由”总开关；保存配置继承对应目标组状态。
   - 胶囊开关白色滑块在开/关两态都按 `top: 50%` 与 Y 轴 `-50%` 居中。
   - OpenAI“上游格式/模型”双列表单以顶部对齐，右侧模型输入框不因左侧帮助文字而下移。
   - Claude / Claude Desktop 的“需要模型映射”开关可交互；关闭后归一化与保存结果仍为 false，生成配置不含角色映射，协议代理对请求模型返回“不改写”；重新开启后原有映射可继续使用。
   - 映射开启时只显示 API 格式和 Sonnet / Opus / Fable / Haiku 映射矩阵，不显示手动模型列表；映射关闭时不显示 API 格式和映射矩阵，只显示“手动指定 Claude Desktop 模型列表（高级，可选）”。
   - 关闭态可获取模型、添加模型、编辑模型 ID、勾选/取消 1M、删除模型；保存后 `modelList` 按每行一项及可选 `[1M]` 后缀序列化，重新打开编辑页可还原。
   - 关闭态手动列表中每条 `[1M]` 声明会在 Claude Desktop 本地模型目录中反映为对应安全模型的 `supports1m=true`；未声明的模型不应被错误标记。
   - 关闭再开启不会丢失 `modelMappingJson` / `modelMapping`，开启再关闭不会丢失 `modelList`；获取模型不会把 `modelMappingEnabled=false` 改回 true。
   - `modelMappingEnabled=true` 时本地模型目录来自 `modelMappingJson`，不含已停用的手动列表；`false` 时目录 ID 等于手动列表中的安全 Claude ID，代理不改写并原样发送。
   - 直接模式拒绝非 `claude-*/anthropic/claude-*` 的 Sonnet / Opus / Haiku / Fable 安全 ID，并提示用户开启模型映射。
   - 获取模型途中切换供应商、映射模式或修改关键配置后，旧响应不会写入当前编辑项。
   - 删除全部手动模型并保存后仍为空；配置 JSON 在眼睛关闭时不出现 Key 原文且不可编辑。
   - 供应商编辑页预设区只渲染 OpenAI 与 Anthropic；DeepSeek、Kimi、Qwen、SiliconFlow、OpenRouter 不在该区域出现。
   - 供应商编辑页不存在可见的“供应商 ID”字段；内部 ID 仍存在于数据模型，新建供应商名称失焦后生成不冲突 ID，编辑已有供应商名称不改变原 ID。

## 必需验证命令

```text
cargo fmt --check
cargo test -p claude-codex-pro-core settings -- --nocapture
cargo test -p claude-codex-pro-core protocol_proxy -- --nocapture
cargo test -p claude-codex-pro-core --test claude_desktop_provider -- --nocapture
cargo test -p claude-codex-pro-manager --test windows_subsystem -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo build -p claude-codex-pro-manager --manifest-path Cargo.toml
```

## 现场验收证据

1. 管理工具设置中能看到三个目标的当前供应商 ID，且值可独立变化。
2. Claude Desktop 当前 `configLibrary` Profile 的检查结果只输出：Profile ID、Key 是否非空、模型数量、本地代理 URL 是否存在；不得输出 Key 原文。
3. 本地代理 `/claude-desktop/v1/models` 返回非空模型目录。
4. 本地代理 `HEAD /claude-desktop` 返回 2xx，`POST /claude-desktop/v1/messages/count_tokens?beta=true` 返回非负 `input_tokens`，日志中不出现 Key 或请求正文。
5. 完全重启 Claude Desktop 后：
   - “提供商设置需要修复”提示消失。
   - 模型选择入口出现。
   - 选择模型后发送最小消息能收到有效回复。
6. 切换 Claude Desktop 前后记录 Codex 当前供应商 ID，二者必须一致。

## 失败条件

- Claude/Claude Desktop 的“使用”仍调用 Codex 切换命令。
- Profile Key 为空或模型数组为空。
- 代理仍读取 Codex 的当前供应商。
- 切换一个目标导致其他目标的当前供应商改变。
- 写入 Claude 设置时覆盖无关字段。
- 仅补出视觉下拉框但模型请求不可用。
- 供应商编辑页仍显示“供应商 ID”输入框，或仍渲染 OpenAI / Anthropic 之外的预设卡片。
- 模型下拉仍使用原生 `<select>`、弹层越过窗口、接口返回列表被默认 Claude 模型污染，或“一键设置”仍因保留全部旧值而无可见效果。
- 编辑表单仍保留第二套路由开关/路由文本，胶囊滑块偏离垂直中心，或 OpenAI 上游格式与模型控件顶边不齐。
- “需要模型映射”无法关闭，视觉关闭但保存后恢复开启，或关闭后代理仍继续改写模型。
- 开启/关闭状态仍显示同一套表单、关闭态没有可编辑手动模型列表，或切换状态导致另一套模型配置丢失。
- “保存并使用”仍先普通保存再应用，导致应用失败时无法恢复修改前设置。
- 编辑当前活动 Claude Desktop 供应商后点击普通“保存”，仍只更新代理读取的 Settings 而没有同步重写 Profile。

## 非范围检查

- 不要求修改 Claude 官方 UI 布局。
- 不要求清理用户已有供应商或 Profile。
- 不要求改变供应商页色调和卡片视觉设计。
