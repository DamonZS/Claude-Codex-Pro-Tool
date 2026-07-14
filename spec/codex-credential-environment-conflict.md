# Codex 凭据环境变量冲突检测与清理

## 背景

Codex 自定义供应商可通过 `config.toml` 中的 `env_key`、`auth.json`、Provider Header 或当前 CCP Profile 提供认证凭据。若 Windows 用户环境中存在同名变量且值与当前活动 Profile 不一致，新启动的 Codex 可能读取错误凭据并收到 `401 Invalid token`。

供应商页目前已有一条静态“检测到 OPENAI 环境变量”提示，但按钮没有真实行为，也无法区分一致、冲突、用户级、系统级与仅当前进程残留。

## 目标

- 在供应商页加载时检测 Codex 当前活动供应商使用的凭据环境变量。
- 比较用户级、系统级和当前进程中的变量与活动 Profile 凭据是否一致。
- 仅在存在环境变量时显示现有紧凑警示条，不改变供应商列表布局。
- 对冲突给出明确原因，并允许用户确认后删除 Windows 用户级变量。
- 清理后提示完全退出并重新启动 Codex。
- 启动 Codex 时从当前 live `config.toml` 解析活动 Provider 的实际 `env_key`，并用 live `auth.json` 的凭据覆盖该子进程变量，避免继承旧值导致 401。
- Windows MSIX Codex 激活前仅在用户级变量缺失或不一致时同步 live 凭据，避免旧注册表值覆盖当前供应商。

## 非目标

- 不静默删除任何环境变量。
- 不删除系统级环境变量，不请求管理员权限。
- 不修改 `CODEX_HOME`。
- 不修改供应商切换、协议转换或上游请求实现。
- 不展示 API Key、Token、完整指纹或其他认证材料。
- 不新增独立设置页或重做供应商 UI。
- 不在通用配置写入 API 或自动化测试中持久修改真实用户凭据环境变量。

## 用户视角

1. 用户打开供应商页。
2. CCP 自动检查当前 Codex 活动 Profile 使用的 `env_key`，默认回退为 `OPENAI_API_KEY`。
3. 没有相关变量时不显示警示条。
4. 有变量且与 Profile 一致时，提示变量存在但未发现值冲突。
5. 有变量且与 Profile 不一致时，提示该变量可能覆盖当前 Profile 并导致 401。
6. 用户点击“删除”后看到确认提示；确认后只删除用户级变量。
7. 删除成功后重新检测，并提示当前 Codex/CCP 进程仍可能保留旧值，需要完全退出后重开。

## 功能要求

### 诊断

- 数据来源包括：
  - 活动 Codex Profile 的已解析 API Key；
  - Provider `env_key`，缺失时使用 `OPENAI_API_KEY`；
  - 当前进程环境变量；
  - Windows 用户级环境变量；
  - Windows 系统级环境变量。
- 返回信息只包含变量名、存在状态、作用域、是否冲突、是否可由 CCP 清理和重启要求。
- 比较使用精确字符串比较；空值视为不存在。
- 只要任一实际环境值与非空 Profile Key 不一致，即标记冲突。
- 若 Profile Key 为空，只报告变量存在，不声称冲突。

### 清理

- 命令只接受诊断返回的变量名格式：ASCII 字母、数字和下划线，且不得为空。
- Windows 仅删除 `HKCU\Environment` 中对应值。
- 不删除 `HKLM` 系统变量。
- 清理成功后广播环境变化通知，使后续新进程可读取新状态。
- 当前进程中继承的旧值不伪装为已消失；结果必须带 `restartRequired=true`。
- 非 Windows 平台不提供持久化删除，返回不可清理的明确结果。

### 启动凭据

- Codex 启动必须从 live `config.toml` 的 `model_provider` 对应表解析 `env_key`；缺失、空值或非法变量名时回退为 `OPENAI_API_KEY`。
- live `auth.json` 中存在非空 `OPENAI_API_KEY` 时，launcher 必须把该值注入活动 Provider 的实际 `env_key`，覆盖从 Manager 或 Windows 继承的旧值。
- 普通可执行文件由 launcher 直接覆盖子进程环境；Windows MSIX 无法接收自定义环境块，因此必须在 `ActivateApplication` 前同步用户级环境变量和当前 launcher 进程环境。
- MSIX 同步必须先比较当前值；用户级变量与 live 凭据一致时不得重复写注册表或重复广播环境变更。
- 用户明确执行“使用”“保存并使用”或现有手动注入动作且配置写入成功后，Manager 必须同步一次 live 凭据；配置写入失败时不得同步。
- Profile 中合法的自定义 `env_key` 必须保留，只有缺失、空值或非法变量名才回退为 `OPENAI_API_KEY`。
- 启动/重启只读取 live 配置，不得切换 Profile，不得重写 `config.toml`、`auth.json` 或 Manager 设置。
- 供应商配置仍只在用户触发“使用”或“保存并使用”时落盘；同一 Profile 重应用不得被旧 live 凭据回填覆盖。
- 日志、状态和测试失败信息不得包含凭据原文。

## UI / 交互要求

- 复用供应商页现有 `.supplier-env-card`、`.supplier-env-chip` 和按钮样式。
- “检测”按钮执行真实重新检测并显示运行反馈。
- “删除”按钮仅在存在用户级变量时启用。
- 删除前使用现有确认交互，不新增复杂弹窗。
- 警示条文案最多两行正文加作用域 Chip，避免增高页面过多。
- 系统级变量存在时显示“系统环境”，但不提供删除能力。

## 数据与接口

- 新增 Tauri 命令：
  - `diagnose_codex_credential_environment`
  - `clear_codex_user_credential_environment`
- 返回统一 `CommandResult`，payload 包含诊断结果。
- 日志只记录变量名、作用域和布尔状态，不记录变量值或指纹。

## 技术约束

- 凭据解析复用现有 `BackendSettings::active_relay_profile()` 及 Profile 配置内容。
- launcher 的 live 配置解析复用 core 的 TOML/JSON 解析能力，不在 launcher 二进制中复制字符串解析逻辑。
- Windows 环境持久化操作位于 core，manager 只负责命令包装。
- `apply_relay_*_to_home*` 等可被测试复用的文件 API 只负责文件，不得隐式写入 `HKCU\\Environment`。
- 自动化测试只能使用 `CCP_TEST_*` 临时变量或纯比较函数，不得写入真实 `OPENAI_API_KEY`。
- 最小改动现有 `SupplierScreen`，不拆分或重构该大组件。
- 不回滚工作区其他改动。

## 交付范围

- core 诊断与用户变量清理模块及测试。
- manager Tauri 命令、命令注册与 TypeScript 类型。
- `AppActions` 调用和供应商页现有警示条接线。
- 定向 Rust 测试、前端类型检查及构建验证。
