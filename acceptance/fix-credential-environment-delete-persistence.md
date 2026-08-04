# Codex 凭据环境变量删除后跨平台持久保持验收标准

对应规格：`spec/fix-credential-environment-delete-persistence.md`

## 平台验收矩阵

| 平台 | 安全清理通过标准 | 启动注入通过标准 | 外部来源与禁止改动 | 必需原生证据 |
|---|---|---|---|---|
| Windows | 删除当前 Manager 进程副本及 `HKCU\Environment` 目标值；不修改 `HKLM` | 普通版仅使用 `Command::env`；MSIX 临时覆盖后在成功、失败路径均恢复 | 不编辑 PowerShell profile、cmd AutoRun、终端或包装脚本；外部/系统来源仍存在时提示无法自动删除 | Windows 定向测试、只读作用域检查、MSIX 恢复测试、Release 可执行文件 |
| macOS | 删除当前 Manager 进程副本及当前 launchd 用户会话目标值 | 使用 `open --env NAME` 和目标命令环境为本次 Codex 启动注入，参数列表不含凭据原文 | 不编辑 shell/profile、LaunchAgent plist、终端或包装脚本；外部来源仍存在时提示无法自动删除 | macOS 原生 launchd 集成测试、目标启动契约测试、Release 构建 |
| Linux | 删除当前 Manager 进程副本；systemd user manager 可用时删除其目标值 | 仅使用 `Command::env` 注入目标 Codex | 不编辑 shell/profile、`environment.d`、桌面会话脚本或系统环境；systemd user manager 不可用和外部来源均准确提示 | Linux 原生 systemd --user 可用环境集成测试；另行覆盖不可用分支；目标启动契约测试；Release 构建 |

## 通用通过标准

1. 供应商切换、Relay 应用、纯 API 应用、状态刷新、设置加载和 CCP 启动在 Windows、macOS、Linux 上均不把 Provider Key 写入 Manager 进程或用户环境。
2. 生产路径不调用持久用户环境同步：不写 `HKCU\Environment`，不执行 `launchctl setenv`，不执行 `systemctl --user set-environment` 或 `import-environment`；Windows MSIX 的可恢复临时作用域除外。
3. 用户确认删除后，当前 Manager 进程副本和平台可管理用户作用域分别返回真实结果；不存在为幂等成功，权限或会话服务错误不能伪装成“不存在”。
4. 删除、诊断、临时注入、日志、测试输出和命令返回值均不包含凭据原文。
5. 变量名严格限制为活动 Provider 的合法 `env_key`，所有系统命令使用结构化参数且不经过 shell 求值。
6. 删除不移除 `auth.json` Key、不停用第三方供应商，也不改回 OpenAI 官方 Provider；通过 CCP 启动的 Codex 仍能获得 live Provider 凭据。
7. Manager 启动独立 launcher 时不注入 Provider Key；launcher 只在最终 Codex 启动边界读取 live 配置并注入目标 Codex。
8. 删除后执行供应商切换、重启 CCP 和启动 Codex，不会由 CCP 再次永久创建用户环境变量。
9. `CredentialEnvironmentDiagnostic` 现有字段保持兼容；新增平台/作用域状态不包含秘密，并能区分已清理、原本不存在、用户会话不可用、失败和外部来源提示。
10. 不修改 `CODEX_HOME`、供应商 URL、模型、协议、Profile 分类、系统提示词或其他无关行为。
11. 变量仅存在于 Manager 进程时删除按钮仍可用；清理当前副本后保留“外部启动环境可能在下次重启重新注入”的提示。

## Windows 通过标准

1. Windows 用户级 `CCP_TEST_*` 变量存在时，删除命令移除 `HKCU\Environment` 中该名称的值和当前 Manager 进程副本，并返回对应作用域不存在。
2. 删除命令不修改同名或其他 `HKLM` 值；只读验证只报告存在性和作用域。
3. 删除后执行一次模拟 MSIX 启动凭据注入，操作内部能读取当前 Provider 凭据，结束后用户级和 launcher 进程变量仍保持删除状态。
4. 临时注入前若用户级或 launcher 进程变量已有其他值，操作结束后精确恢复各自原值，不保留 Provider 凭据。
5. 激活、闭包或目标操作返回错误时仍执行用户级和进程级恢复；恢复失败作为错误返回。
6. `ActivateApplication` 只在新 MSIX 进程创建路径使用临时注入；复用已有 Codex/CDP 时不写注册表。
7. 不同 launcher 进程的临时 MSIX 注入按“保存、激活、恢复”完整作用域串行执行，不把另一次激活的临时值误当成原值。
8. 用户删除与临时 MSIX 注入使用同一个跨进程锁；删除发生在注入期间时，必须等待恢复完成后再删除，最终不得恢复出删除前的旧值。
8. MSIX 激活窗口内，live 配置中所有非活动 Provider 的合法 `env_key` 均不存在，只设置活动 Provider 的 key；窗口结束后所有值和注册表类型逐项恢复。

## macOS 通过标准

1. 当前 launchd 用户会话中存在唯一 `CCP_TEST_*` 变量时，删除命令移除该会话值和当前 Manager 进程副本；再次查询用户会话时目标变量不存在。
2. launchd 会话中变量原本不存在时删除幂等成功；会话不可访问或命令失败时返回真实错误。
3. 供应商切换、配置应用、CCP 重启和 Codex 启动均不执行 `launchctl setenv`，也不把 Provider Key 写入 Manager 环境。
4. Codex 启动只通过 `open --env NAME` 与目标命令环境注入凭据，`open` 参数不含凭据原文；启动完成后 launchd 用户会话仍保持删除状态。
5. 从带有测试变量的外部 shell/包装进程启动 CCP 时，删除不会编辑任何 profile 或 plist；若变量在下一次相同外部启动中重现，界面提示“外部启动环境可能仍在注入，CCP 无法自动删除”。

## Linux 通过标准

1. systemd user manager 可用且包含唯一 `CCP_TEST_*` 变量时，删除命令移除 user manager 值和当前 Manager 进程副本；再次查询 user manager 环境时目标变量不存在。
2. user manager 中变量原本不存在时删除幂等成功；user manager 不存在、未启动或不可连接时返回明确不可用状态，不回退到编辑 profile、`environment.d` 或系统文件。
3. 供应商切换、配置应用、CCP 重启和 Codex 启动均不执行 `systemctl --user set-environment`、`import-environment` 或等价写回，也不把 Provider Key 写入 Manager 环境。
4. Codex 直接启动继续只通过 `Command::env` 注入目标凭据，启动完成后 systemd user manager 环境仍保持删除状态。
5. 从带有测试变量的外部 shell/包装进程启动 CCP 时，删除不会编辑任何 profile、`environment.d` 或启动脚本；若变量在下一次相同外部启动中重现，界面提示“外部启动环境可能仍在注入，CCP 无法自动删除”。

## 外部来源与 UI 通过标准

- 删除确认文案准确列出当前平台会清理的 Manager 进程和用户作用域，并说明不会删除 `auth.json` 凭据。
- 删除结果按作用域区分已清理、原本不存在、不可用和失败；部分成功不得显示为完全成功。
- 只有进程继承值、变量在相同外部启动后重现，或系统级/外部作用域仍存在时，UI 明确提示 CCP 无法自动删除 shell/profile/外部启动来源。
- 仅当前 Manager 进程副本存在时删除按钮可清理该副本，但外部来源提示不得因此消失；删除结果保留重启要求和外部来源状态。
- 提示可以列出平台相关人工检查方向，但不得声称已定位到未经验证的具体文件，不得自动打开或修改这些文件。
- macOS/Linux 用户会话工具不可用时提供可操作错误，不要求用户授予 sudo/管理员权限。

## 必需验证

### 跨平台公共证据

- 运行 core `credential_environment` 单元/契约测试，覆盖合法变量名、非法变量名、原本不存在、成功删除、部分失败、错误传播和凭据不泄露。
- 运行 core launcher 定向测试，证明 Manager 不携带 Provider Key，普通 Windows/Linux 直接启动使用 `Command::env`，macOS 使用 `open --env`，Windows MSIX 单独验证临时注入与恢复，且不存在持久用户环境写回。
- 运行 Manager 供应商命令定向测试，证明供应商切换、Relay 应用和纯 API 应用成功路径不修改 Manager 或用户环境。
- 运行外部来源提示测试，使用临时启动环境和临时 HOME/profile 哨兵文件，证明变量重现时显示无法自动删除，同时所有 profile、plist、`environment.d` 和启动脚本内容保持不变。
- 运行 `npm --prefix apps/claude-codex-pro-manager run check`、`cargo fmt --check` 及与改动范围匹配的 Rust 测试。
- 检查源码契约或调用图，证明生产代码没有 Provider Key 的 `std::env::set_var`、`launchctl setenv`、`systemctl --user set-environment/import-environment` 持久写入路径；测试夹具建立临时状态的调用必须与生产路径区分。

### Windows 原生证据

- 先运行能复现旧行为的 Windows 定向测试，并记录修复前删除后被持久写回的失败证据。
- 使用唯一 `CCP_TEST_*` 值运行 `HKCU\Environment` 删除、Manager 进程清理、MSIX 无原值/已有原值/操作失败恢复、双 launcher 并发和“启动期间删除”串行化测试。
- 只读检查 `HKLM` 未改变；不得打印值。
- 生成或验证默认 `target/release/claude-codex-pro.exe`，供实际测试。

### macOS 原生证据

- 在 macOS 原生 runner/机器上，用测试夹具临时执行 `launchctl setenv CCP_TEST_* <sentinel>` 建立会话状态；调用产品删除逻辑后，以 `launchctl getenv` 仅验证目标值不存在，并在测试结束时恢复测试前状态。
- 运行 macOS `open --env NAME` 启动契约/集成测试，确认目标进程能获得测试凭据、命令参数不含凭据原文且 launchd 用户会话未被重新创建。
- 生成或验证默认 `target/release/claude-codex-pro` 或对应 macOS Release 应用产物。

### Linux 原生证据

- 在带可用 systemd user manager 的 Linux 原生 runner/机器上，用测试夹具临时执行 `systemctl --user set-environment CCP_TEST_*=<sentinel>` 建立状态；调用产品删除逻辑后，以 `systemctl --user show-environment` 仅验证目标名称不存在，并在测试结束时恢复测试前状态。
- 另行验证 systemd user manager 不可用分支，确认返回不可用状态且未修改任何 shell/profile 或环境配置文件。
- 运行 Linux `Command::env` 目标进程注入测试，并生成或验证默认 `target/release/claude-codex-pro`。

原生用户会话集成验证不能由交叉编译替代。若当前环境缺少 macOS、Linux 或可用的 systemd user manager，必须把对应平台标记为“未完成原生验收”，列出已通过的契约/构建检查和剩余风险，不得宣称三平台全部通过。

## 完成证据

- 每个平台的测试命令、原生运行环境、退出状态和关键通过项。
- 删除前后仅报告测试变量的存在性、作用域和状态，不打印 sentinel 或真实 API Key。
- profile/plist/`environment.d` 哨兵文件的删除前后摘要或哈希一致证据。
- `git diff` 中仅有本任务必要改动以及工作区原有改动；不得回滚用户文件。
- Windows、macOS、Linux 平台矩阵逐项标记已通过、未验证或失败；未验证项附原因。

## 失败条件

- 任一平台在删除后因供应商切换、CCP 重启或 Codex 启动再次由 CCP 永久创建 Manager 或用户环境变量。
- 为解决问题而停用第三方供应商、删除 `auth.json` Key 或改回 OpenAI 官方 Provider。
- Windows MSIX 激活后没有恢复用户和 launcher 原环境状态。
- macOS/Linux 为清理用户会话而编辑 shell/profile、plist、`environment.d`、系统文件，或要求提权扩大删除范围。
- 外部来源仍会注入变量时宣称已经彻底删除，或未提示 CCP 无法自动删除。
- 用户会话清理失败、服务不可用或部分成功被显示为完全成功。
- 任何日志、测试输出、文档或 UI 暴露真实凭据。

## 非目标检查

- 不验证第三方上游服务本身是否在线。
- 不自动操作用户真实的 `OPENAI_API_KEY`。
- 不要求自动重启当前运行中的 Codex，也不要求终止已运行的其他应用来移除其环境副本。
