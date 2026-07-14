# Codex 凭据环境变量冲突检测与清理验收标准

对应规格：`spec/codex-credential-environment-conflict.md`

## 通过标准

1. 无相关环境变量时，诊断返回 `present=false`，供应商页不显示警示条。
2. 用户或进程变量与活动 Profile Key 一致时，诊断返回存在但 `conflict=false`。
3. 任一环境变量与活动 Profile Key 不一致时，诊断返回 `conflict=true`，且返回体不含任何凭据原文。
4. Windows 用户级变量存在时，`canClearUser=true`；仅系统级或仅进程变量存在时不得声称可清理。
5. 清理命令拒绝非法变量名，只删除 `HKCU\Environment` 对应变量，不触碰系统变量和 `CODEX_HOME`。
6. 清理成功返回 `restartRequired=true`，UI 明确提示完全退出并重启 Codex。
7. 供应商页复用现有警示条，不新增页面，不改变供应商卡片和筛选布局。
8. “检测”和“删除”按钮均连接真实命令；删除前有确认。
9. 系统级变量仅提示，不执行提权或静默删除。
10. 日志和命令返回均不包含 Key、Token 或完整凭据指纹。
11. 自定义 Provider `env_key` 启动时获得 live `auth.json` 中的当前凭据，继承的旧同名变量被覆盖。
12. Provider 未声明有效 `env_key` 时回退为 `OPENAI_API_KEY`。
13. 启动/重启前后的 `config.toml`、`auth.json` 与 Manager 设置内容保持不变，不发生隐式 Profile 切换或重复写入。
14. 同一活动 Profile 重新应用时，用户刚提交的凭据不会被旧 live `auth.json` 回填覆盖。
15. 合法自定义 `env_key` 在 Profile 规范化和应用后保持不变，不被强制改写为 `OPENAI_API_KEY`。
16. 通用 relay 文件应用 API 不写入用户注册表或真实凭据环境变量；集成测试运行前后不会改变真实 `OPENAI_API_KEY`。
17. Windows MSIX 激活前会把 live 凭据同步到活动 Provider 的用户级环境变量；值已一致时不重复写注册表。
18. 供应商切换或手动注入成功后执行同步，写入失败时不执行；同步结果与日志不包含凭据原文。

## 验证方式与证据

- Rust 单元测试覆盖纯诊断比较、变量名校验及无凭据泄漏。
- core 单元测试覆盖活动 Provider `env_key` 解析、默认回退与 live `auth.json` 凭据组合。
- core 测试覆盖同步决策、相同值不写、不同值更新、自定义 `env_key` 保留，以及通用文件 API 无环境副作用。
- launcher 生命周期测试覆盖启动只读配置和自定义 `env_key` 子进程环境覆盖契约。
- launcher 契约测试覆盖 MSIX 激活前调用 live 凭据同步。
- relay switch 测试覆盖同一 Profile 重应用不回填旧 live 凭据。
- Windows 定向测试使用临时测试变量验证用户环境变量写入、检测、删除，不使用真实 `OPENAI_API_KEY`。
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
