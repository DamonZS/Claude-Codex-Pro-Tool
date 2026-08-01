# Codex 供应商路由关闭与新建配置持久化

## 背景

Windows 与 macOS 共用的供应商前端在关闭当前 Codex 路由时，只撤销运行中的代理配置，没有保存已经计算出的 `routeEnabled: false`，因此界面提示关闭后又恢复为开启。

新建供应商还存在两个容易造成“保存后配置不见了”的交互问题：新 Profile 的“保存并使用”按钮被限制为仅编辑已有 Profile 时可用；选择会改变 `targetApp` 的官方预设后，保存会关闭编辑器但仍停留在原目标标签，导致刚保存的卡片在当前列表中不可见。API Key 与 Base URL 又采用 `authContents`、`configContents`、`upstreamBaseUrl` 持久化并在加载时还原，必须有明确的往返测试防止后续改动破坏。

## 目标

- 关闭当前 Codex 路由时，同时撤销运行配置并持久化关闭状态。
- 新建普通供应商时允许一次点击“保存并使用”，完成保存、切换和当前供应商更新。
- 保存后将列表切换到已保存 Profile 的目标应用标签，确保卡片立即可见。
- Codex 新建供应商的 API Key 与 Base URL 保存、加载后保持一致。
- 从 CC Switch 导入后被旧状态误标为官方登录的 Codex Profile，在用户填写第三方
  Base URL 后必须自动恢复为纯 API Profile，不能在保存规范化时清空接口地址。
- Codex 供应商开启路由时，无论上游使用 Responses 还是 Chat Completions，
  Codex 的活动 provider 都指向 CCP 本地代理；上游 Base URL 只由代理读取并转发。

## 非目标

- 不改变目标级路由总开关的现有分组语义。
- 不在编辑表单重新增加第二套路由开关。
- 不修改用户现有供应商、API Key、活动供应商或官方客户端配置。
- 不改变 Claude 与 Claude Desktop 的代理协议或模型映射规则。

## 用户视角

1. 用户在 Codex 标签关闭路由，界面先显示处理中；运行配置撤销且设置保存成功后，开关保持关闭。
2. 用户点击“添加供应商”，填写名称、API Key 与请求地址，可以直接点击“保存并使用”。
3. 如果用户选择 Anthropic 等会改变目标应用的预设，保存完成后列表自动进入对应的 Claude 或 Claude Desktop 标签，并显示刚保存的供应商。
4. 用户重新打开该供应商时，API Key 与请求地址仍然存在；密钥继续按现有规则隐藏显示，不写入日志。

## 功能要求

- Codex 活动路由关闭分支必须按顺序执行：`clearRelayMode` 成功、`saveSupplierSettings` 成功、显示完成提示。
- 任一步失败时不得显示“已关闭”完成提示。
- 保存的目标 Profile 必须写入 `routeEnabled: false` 和对应 Direct 路由状态。
- 普通新建 Profile 的“保存并使用”不得依赖 `editingExisting`；聚合供应商仍不得直接应用。
- `saveDraft` 成功关闭编辑器前，必须把 `supplierTargetFilter` 更新为 `savedProfile.targetApp` 对应目标。
- 保存成功提示继续使用已保存 Profile 的名称或 ID，不展示 API Key。
- Codex Profile 的 `apiKey`、`baseUrl` 可以是运行时字段，但 `SettingsStore` 保存加载往返必须通过 `authContents`、`configContents`、`upstreamBaseUrl` 恢复相同值。
- `importSource: "cc-switch"` 的 Codex Profile 不得仅凭 `relayMode: "official"`
  享受官方登录的空地址豁免；存在第三方 Base URL 时必须以 `pureApi` 持久化并生成
  完整 provider 配置。
- `routeEnabled: true` 时，写入 Codex `config.toml` 的 provider `base_url` 必须是 CCP
  本地 Responses 代理地址。Responses 上游透明转发，Chat Completions 上游执行现有协议转换。
- `routeEnabled: false` 时保留现有直连写入行为，不强制经过本地代理。

## UI / 交互要求

- 路由关闭继续显示运行中、成功或失败通知。
- 新建普通供应商的“保存并使用”按钮可点击；保存期间仍禁用，防止重复提交。
- 保存到其他目标标签时不新增额外弹窗，直接切换标签并显示卡片。
- API Key 仍使用密码输入和眼睛按钮，不在卡片、通知或日志中显示明文。
- 供应商编辑器打开时，页面级 `ops-screen` 不再承担纵向滚动，窗口最右侧不得出现页面滚动条。
- 编辑器保持“固定标题栏、可滚动表单主体、固定底部操作栏”的三行结构；仅中间表单主体允许纵向滚动。
- “取消”“保存”“保存并使用”始终完整显示在编辑器底部，滚动长表单时位置不变，表单内容不得被操作栏遮挡。
- 在桌面窗口和项目支持的最小窗口宽度下，底部状态文本可以截断，但三个操作按钮不得换出可视区域。

## 技术约束

- 复用现有 `saveSupplierSettings`、`switchSupplierProfile`、`withSupplierRoutingState` 和 `SettingsStore`。
- 不引入新依赖，不改变 Tauri 命令协议。
- Windows 与 macOS 使用同一 React 逻辑，修复不得加入平台特判。

## 交付范围

- 供应商页面路由关闭、保存和目标标签切换逻辑。
- Manager 前端源码契约测试。
- Core Codex 供应商凭据与 URL 设置往返测试。
- TypeScript 检查、前端构建、Rust 定向测试和默认 Release 构建。
