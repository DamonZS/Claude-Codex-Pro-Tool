# Codex 凭据环境变量删除后跨平台持久保持

## 背景

CCP 的供应商页允许用户检测活动 Codex Provider 对应的凭据环境变量并执行删除。旧实现虽然能删除 Windows `HKCU\Environment` 中的值，但供应商切换、配置应用和启动链路又可能从 live `auth.json` 把 Provider Key 写入长期存活的 Manager 进程或用户环境，导致用户重启 CCP、刷新状态或再次启动 Codex 后仍看到该变量。

该问题不只影响 Windows。macOS 应用可能从当前 launchd 用户会话继承变量，Linux 桌面应用或用户服务可能从 systemd user manager 继承变量；从终端、shell profile、环境配置文件、启动脚本或其他外部启动器继承的变量，也可能在 CCP 重启时再次出现。

`spec/codex-credential-environment-conflict.md` 已要求凭据只在 Codex 启动边界提供。本规格将这一边界统一到 Windows、macOS 和 Linux：CCP 可以清理自己能够安全定位的当前用户作用域，但不能把 Provider Key 持久回写到 Manager 或用户环境，也不能为了删除一个变量而猜测并修改用户的 shell/profile 文件。

## 术语与责任边界

- **Manager 进程环境**：当前 CCP Manager 进程持有的环境副本。删除动作可以移除其中的目标变量；供应商切换、配置应用、状态刷新和设置加载不得把 Provider Key 写入其中。
- **CCP 可安全清理的用户作用域**：Windows 当前用户的 `HKCU\Environment`、macOS 当前登录用户的 launchd 会话环境、Linux 当前用户的 systemd user manager 环境。
- **外部来源**：shell/profile、系统级环境、LaunchAgent 或用户自定义 plist、`environment.d` 配置、桌面会话脚本、终端设置、包装脚本以及其他不由 CCP 管理的启动器。CCP 可以检测到变量仍由当前进程继承，但不能可靠确定或自动编辑这些来源。
- **目标进程注入**：仅向本次新启动的 Codex 目标进程提供凭据，不改变 Manager、用户会话或系统的长期环境。

## 目标

- Windows、macOS 和 Linux 上的供应商切换、配置应用、状态刷新、设置加载及 CCP 重启均不得把 Provider Key 持久写入 Manager 进程环境或用户环境。
- 用户确认删除后，CCP 安全清理当前平台可管理的用户作用域及当前 Manager 进程副本；后续供应商切换、CCP 重启或 Codex 启动不得由 CCP 再次永久写回。
- Windows 支持清理 `HKCU\Environment`；macOS 支持清理当前 launchd 用户会话；Linux 支持清理当前 systemd user manager 环境。
- 外部 shell/profile 等来源不能安全自动删除时，界面明确说明限制与人工检查方向，不虚假宣称已彻底删除。
- 已保存的第三方供应商继续使用 live `config.toml` 与 `auth.json` 中的当前凭据启动 Codex。
- 所有平台仅在 Codex 启动边界执行目标进程范围的凭据注入；Windows MSIX 所需的临时用户环境覆盖必须完整恢复。
- UI、日志、测试输出和文档均不得包含真实 API Key。

## 非目标

- 不自动删除用户尚未主动确认删除的环境变量。
- 不编辑 `.zshrc`、`.zprofile`、`.bashrc`、`.bash_profile`、`.profile`、PowerShell profile 或其他 shell/profile 文件。
- 不编辑 LaunchAgent plist、`~/.config/environment.d/*`、桌面会话脚本、终端配置、包装脚本或其他外部持久化来源。
- 不删除或修改 Windows `HKLM`、macOS 系统 launchd 域、Linux system manager、`/etc/environment` 等系统级环境。
- 不通过提权、`sudo` 或管理员权限扩大删除范围。
- 不修改 `CODEX_HOME`、供应商 URL、模型、协议和 Profile 分类。
- 不改变 Codex 官方登录凭据与 ChatGPT 登录模式，也不删除 `auth.json` 中的第三方 Provider Key。
- 不在本任务中重构供应商页面布局。

## 用户视角

1. 用户在供应商页看到活动 Provider 的凭据环境变量状态，以及 CCP 能安全清理的当前平台作用域。
2. 用户点击“删除”并确认后，CCP 清理当前 Manager 进程副本和平台对应的用户作用域。
3. 删除结果逐项显示已清理、原本不存在、无法访问或仍可能由外部来源注入，不把部分成功描述成全部成功。
4. 用户关闭并重新打开 CCP，变量不再因为 CCP 自动写回而出现。
5. 用户通过 CCP 启动 Codex，所选供应商仍从保存的配置获得凭据；Codex 启动后用户作用域仍保持删除状态。
6. 若变量来自 shell/profile 或其他外部来源，CCP 明确提示无法自动删除，并建议用户检查相应启动环境；CCP 不改写这些文件。
7. 若用户没有执行删除，任何临时启动注入结束后都必须恢复原值，不能把原值永久替换成 Provider Key。

## 平台行为矩阵

| 平台 | CCP 可安全清理的作用域 | Codex 凭据注入方式 | CCP 禁止修改的外部/系统来源 | 删除后仍出现时的行为 |
|---|---|---|---|---|
| Windows | 当前 Manager 进程副本；`HKCU\Environment` 中目标 `env_key` | 普通可执行版使用 `Command::env`；MSIX 激活使用最短作用域临时覆盖并恢复 | `HKLM`、PowerShell profile、cmd AutoRun、终端/包装脚本及其他启动器配置 | 标记为系统级或外部启动环境可能仍在注入，提示人工检查，不自动删除 |
| macOS | 当前 Manager 进程副本；当前登录用户 launchd 会话中的目标 `env_key`，等价于安全调用 `launchctl unsetenv` | 使用 `open --env NAME` 并通过目标 `Command` 环境只为本次 Codex 启动注入；凭据不得进入进程参数 | 系统 launchd 域、LaunchAgent plist、shell/profile、终端设置和包装脚本 | 标记为外部启动环境可能仍在注入，提示人工检查，不编辑 plist/profile |
| Linux | 当前 Manager 进程副本；可用时当前用户 systemd user manager 中的目标 `env_key`，等价于安全调用 `systemctl --user unset-environment` | 直接启动时使用 `Command::env` 只注入目标 Codex | system manager、`/etc/environment`、shell/profile、`~/.config/environment.d/*`、桌面会话脚本和包装脚本 | systemd user manager 不可用时明确报告；外部来源提示人工检查，不编辑配置文件 |

## 功能要求

### 跨平台持久化边界

- 供应商切换、Relay 应用、纯 API 应用、状态刷新、设置加载和 CCP 启动只能更新 CCP/Codex 配置文件，不得创建或更新任何平台的用户凭据环境。
- 上述路径不得调用 `std::env::set_var` 或等价 API 把 Provider Key 写入长期存活的 Manager 进程。
- 上述路径不得写入 `HKCU\Environment`，不得执行 `launchctl setenv`，不得执行 `systemctl --user set-environment`、`systemctl --user import-environment` 或其他持久用户会话同步。
- Manager 启动独立 launcher 时不得携带 Provider Key；launcher 在最终 Codex 启动边界重新读取 live 配置并只注入目标 Codex。
- 删除动作允许移除当前 Manager 进程中的目标变量，但不得用 live `auth.json` 的值填充 Manager 环境。

### 通用删除规则

- 删除仅作用于活动 Provider 的合法 `env_key`；变量名必须经过现有严格校验，不能作为 shell 文本拼接或任意命令输入。
- 平台适配器可在 core 内部读取值以完成存在性判断或冲突比较，但凭据原文不得越过 core 边界、持久化、返回或记录；删除结果只暴露存在性、作用域和状态。
- 每个平台分别返回当前进程和可管理用户作用域的清理结果；任一作用域失败时保留真实错误，不能把部分成功报告为完全成功。
- 用户作用域原本不存在时视为幂等成功；权限不足、会话服务不可用或命令执行失败不得伪装成“不存在”。
- 清理完成后重新诊断。若只有当前进程继承值、变量重启后重现，或存在非 CCP 管理的作用域，必须显示“可能来自外部 shell/profile/启动环境，CCP 无法自动删除”的提示。
- CCP 不尝试猜测具体 profile 文件，也不自动修改任何外部来源。

### Windows 用户环境与 MSIX 启动

- 删除命令清理当前 Manager 进程副本和 `HKCU\Environment` 中同名值，不修改 `HKLM`。
- 启动器从 live `config.toml` 解析活动 Provider 的 `env_key`，从 live `auth.json` 读取非空 `OPENAI_API_KEY`。
- 调用 `IApplicationActivationManager::ActivateApplication` 前，保存用户级与当前 launcher 进程中同名变量的原状态。
- 临时清除 live `config.toml` 中所有非活动 Provider 声明的合法 `env_key`，只为活动 Provider 设置当前凭据；激活结束后逐项恢复原状态。
- 仅为本次激活临时设置当前凭据；`ActivateApplication` 返回后立即恢复原值，原本不存在时删除临时值。
- 激活失败、闭包返回错误或恢复路径提前退出时也必须执行恢复；恢复失败必须作为错误返回并记录不含凭据原文的诊断信息。
- 已有 Codex/CDP 进程被复用时不得临时改写环境，因为没有创建新进程。
- Windows 删除动作与所有 launcher 的临时注入共享跨进程命名互斥体；删除必须等待正在进行的临时注入恢复完成后再落地，不能被 launcher 随后恢复成旧值。

### macOS launchd 用户会话

- 删除命令移除当前 Manager 进程副本，并在当前登录用户的 launchd 会话中对合法变量名执行等价于 `launchctl unsetenv <env_key>` 的结构化调用。
- 生产代码不得通过 shell 字符串拼接执行命令，不得调用 `launchctl setenv` 写回 Provider Key。
- launchd 会话中变量原本不存在时幂等成功；当前会话不可访问或清理失败时显示对应错误。
- 清理 launchd 会话不会修改 shell/profile、LaunchAgent plist 或已经运行的其他进程；界面必须准确说明这一边界。

### Linux systemd user manager

- 删除命令移除当前 Manager 进程副本，并在 systemd user manager 可用时对合法变量名执行等价于 `systemctl --user unset-environment <env_key>` 的结构化调用。
- 生产代码不得通过 shell 字符串拼接执行命令，不得调用 `set-environment`、`import-environment` 或其他方式写回 Provider Key。
- systemd user manager 不存在、未启动或不可连接时，返回明确的“用户会话环境不可用”状态，不以编辑 shell/profile、`environment.d` 或系统文件作为回退。
- 清理 systemd user manager 不会修改 shell/profile、`environment.d`、桌面会话脚本或已经运行的其他进程；界面必须准确说明这一边界。

### 目标 Codex 启动

- 普通 Windows 和 Linux 直接可执行启动继续使用 `Command::env` 为目标 Codex 子进程注入活动 Provider 凭据。
- macOS 通过 `open --env NAME` 与 `Command::env(NAME, credential)` 为本次 Codex 启动注入凭据；凭据原文不得出现在 `open` 参数列表。
- Windows MSIX 仅允许使用上述可恢复的临时激活作用域，不得保留用户环境改动。
- 所有启动方式都不得把凭据写入日志、命令返回体、崩溃诊断或可见参数预览。

## UI / 交互要求

- 保留供应商页现有环境变量检测、删除按钮、确认交互和紧凑提示样式，并使删除能力在 Windows、macOS 和 Linux 上可用。
- 当前 Manager 进程副本或平台可管理的用户会话变量存在时允许删除；仅进程来源即使可删除当前副本，也必须提示相同外部启动环境可能在下次启动时重新注入。
- 确认文案列出本平台将清理的作用域，并明确不会删除 `auth.json` 凭据或停用供应商。
- 删除后按作用域显示结果，不得声称已修改系统级环境、已编辑 shell/profile 或已重启 Codex。
- 当变量可能来自外部 shell/profile/启动器时，显示无法自动删除的提示和平台相关人工检查方向；不得声称已定位到未经验证的具体文件。
- systemd user manager、launchd 会话或 Windows 用户环境访问失败时显示可操作错误，不把失败显示为“已删除”。
- 不新增显示 API Key、Token 或凭据指纹的字段、通知或调试信息。

## 数据与接口要求

- 不新增包含 API Key 的前端字段或 Tauri 返回字段。
- 删除结果应能表达平台、进程作用域、用户作用域、外部来源提示及错误状态；如扩展 `CredentialEnvironmentDiagnostic`，新增字段必须可选并保持现有字段兼容。
- 临时注入函数的返回值只表达完成状态或错误，不携带凭据。
- 现有供应商 Profile、`config.toml` 和 `auth.json` 格式保持兼容。

## 技术与安全约束

- 平台环境操作保留在 core 的凭据/系统集成边界内，Manager 命令层只调用结构化接口。
- Windows 注册表、macOS `launchctl` 和 Linux `systemctl` 均使用结构化参数；变量名不得经过 shell 求值。
- Windows MSIX 使用作用域守卫或等价的结构化恢复逻辑，并以跨进程命名互斥体串行覆盖保存、激活、恢复和用户删除全过程。
- macOS/Linux 删除不得通过提升权限扩大到系统作用域；用户会话工具不可用时应停止并报告。
- 测试只能使用唯一的 `CCP_TEST_*` 变量，并在结束时恢复测试前状态；不得操作用户真实 `OPENAI_API_KEY`。
- 不回滚工作区已有供应商、主题、系统提示词或注入相关改动。

## 交付范围

- core 跨平台凭据环境诊断与安全清理适配。
- Windows MSIX 临时注入与恢复、普通 Windows/macOS/Linux 目标进程注入。
- Manager 供应商写入链路移除所有平台的持久凭据同步，并提供跨平台删除状态与外部来源提示。
- 覆盖 Windows、macOS、Linux 的单元/契约测试，以及各原生平台用户会话环境的集成验证。
- 对应验收文档与不含真实凭据的验证证据。
