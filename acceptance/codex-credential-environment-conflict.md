# Codex 凭据环境变量冲突检测与清理验收标准

对应规格：`spec/codex-credential-environment-conflict.md`

## 通过标准

1. 无相关环境变量时，诊断返回 `present=false`，供应商页不显示警示条。
2. 用户或进程变量与活动 Profile Key 一致时，诊断返回存在但 `conflict=false`。
3. 任一环境变量与活动 Profile Key 不一致时，诊断返回 `conflict=true`，且返回体不含任何凭据原文。
4. 当前 Manager 进程副本或当前平台可管理的用户会话变量存在时 `canClearUser=true`；仅进程来源同时返回外部来源提示，仅系统级变量存在时不得声称可自动清理。
5. 清理命令拒绝非法变量名，并按平台清理 Manager 进程副本及 `HKCU\Environment`、launchd 用户会话或 systemd user manager，不触碰系统级或外部配置及 `CODEX_HOME`。
6. 清理成功返回 `restartRequired=true`，该字段只表示已运行的 Codex 或其他进程仍需重启，不得用来声称 Manager 进程副本未清理。
7. 供应商页复用现有警示条，不新增页面，不改变供应商卡片和筛选布局。
8. “检测”和“删除”按钮均连接真实命令；删除前有确认。
9. 系统级变量仅提示，不执行提权或静默删除。
10. 日志和命令返回均不包含 Key、Token 或完整凭据指纹。
11. 自定义 Provider `env_key` 启动时获得 live `auth.json` 中的当前凭据，继承的旧同名变量被覆盖。
12. Provider 未声明有效 `env_key` 时回退为 `OPENAI_API_KEY`。
13. 启动/重启前后的 `config.toml`、`auth.json` 与 Manager 设置内容保持不变，不发生隐式 Profile 切换或重复写入。
14. 同一活动 Profile 重新应用时，用户刚提交的凭据不会被旧 live `auth.json` 回填覆盖。
15. 合法自定义 `env_key` 在 Profile 规范化和应用后保持不变，不被强制改写为 `OPENAI_API_KEY`。
16. 通用 relay 文件应用 API 不写入 Manager 进程、三平台用户会话或真实凭据环境变量；集成测试运行前后不会改变真实 `OPENAI_API_KEY`。
17. Windows MSIX 激活只在进程创建作用域内临时提供 live 凭据，激活结束或失败后精确恢复用户级和 launcher 进程环境。
18. 所有平台的供应商切换或手动注入只写配置文件，不修改 Manager 进程或用户持久环境；启动凭据只在 core launcher 的最终 Codex 启动边界注入且不出现在日志中。
19. Windows 删除与 MSIX 临时注入跨进程序列化；删除发生在临时注入期间时，最终状态仍为已删除，不会被 launcher 恢复旧值。

## 验证方式与证据

- Rust 单元测试覆盖纯诊断比较、变量名校验及无凭据泄漏。
- core 单元测试覆盖活动 Provider `env_key` 解析、默认回退与 live `auth.json` 凭据组合。
- core 测试覆盖 MSIX 临时注入与恢复、自定义 `env_key` 保留，以及通用文件 API 无环境副作用。
- launcher 生命周期测试覆盖启动只读配置和自定义 `env_key` 子进程环境覆盖契约。
- launcher 契约测试覆盖 MSIX 激活使用作用域临时凭据且不持久同步。
- relay switch 测试覆盖同一 Profile 重应用不回填旧 live 凭据。
- Windows 定向测试使用临时测试变量验证用户环境变量写入、检测、删除，不使用真实 `OPENAI_API_KEY`。
- macOS 原生测试使用临时 `CCP_TEST_*` 变量验证 launchd 用户会话检测和删除；Linux 原生测试验证 systemd user manager 的可用与不可用分支。
- 外部来源契约测试证明 CCP 不编辑 shell/profile、plist、`environment.d` 或启动脚本，并显示人工检查提示。
- `npm --prefix apps/claude-codex-pro-manager run check` 通过。
- `npm --prefix apps/claude-codex-pro-manager run vite:build` 通过。
- `cargo fmt --check` 通过。
- 检查 Git diff，确认没有 UI 大改和无关重构。

## 失败条件

- 自动或静默删除变量。
- 删除系统级变量或要求提权。
- 页面/日志/返回值出现真实凭据。
- 清理后宣称当前已运行 Codex 立即生效。
- 静态按钮仍未调用真实后端命令。

## 非目标检查

- 不验证上游 API Key 本身是否有效。
- 不自动重启 Codex。
- 不修改第三方供应商配置或模型列表。
