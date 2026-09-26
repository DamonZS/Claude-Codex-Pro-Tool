# 系统提示词组合预览与部署验收

对应规格：`spec/prompt-composition-deployment.md`。

## 必须通过

- [x] Core 组合器单测覆盖顺序、去重、空输入、重复 ID、单源/总大小上限和稳定哈希。
- [x] Tauri 组合预览命令只返回数据，不写客户端文件。
- [x] 页面显示组合来源、警告和摘要，并能把预览应用到目标。
- [x] 组合预览后沿用现有逐目标投放与还原路径。
- [x] 前端类型检查、组合器测试、现有投放测试和 `git diff --check` 通过。

## 验证记录

- `cargo test -p claude-codex-pro-core --lib`：753 passed, 0 failed, 7 ignored。
- `cargo fmt --check`：通过。
- `npm --prefix apps/claude-codex-pro-manager run check`：通过。
- `node scripts/build-prompt-index.mjs --check`：通过（23 个提示词、2 个技能、2 个工具包）。
- 7 个 ignored 测试均要求本机正在运行 Claude Desktop，当前环境未启动该进程，因此保持跳过。

## 不在本次验收范围

- 不对来源提示词的实际模型效果作保证。
- 不执行真实第三方客户端的线上请求验证。
