# 验收标准：Prompt 索引跨平台一致性修复

验证对象：`spec/fix-prompt-index-cross-platform-consistency.md`

## 验收项

1. 工具文本资源跨平台保持 LF。
   - `git check-attr -a` 对 `Prompt/tools` 文本文件返回 `text: set` 与 `eol: lf`。
   - ZIP 文件返回 `binary: set`。

2. 索引校验在 Windows 工作树通过。
   - `node scripts/build-prompt-index.mjs --check` 退出码为 0。

3. 索引校验在规范 LF 副本通过。
   - 独立 LF 检出副本执行同一命令退出码为 0。

4. 变更质量与工作流结果。
   - `git diff --check` 退出码为 0。
   - 推送提交触发的 `Security Audit` 中 `Prompt Index Consistency` 成功。

## 非目标

- 不要求 cargo、前端或安装器构建作为本次索引 CI 修复的验收条件。
- 不修改工具包内容本身。
