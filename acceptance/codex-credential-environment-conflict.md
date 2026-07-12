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

## 验证方式与证据

- Rust 单元测试覆盖纯诊断比较、变量名校验及无凭据泄漏。
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
