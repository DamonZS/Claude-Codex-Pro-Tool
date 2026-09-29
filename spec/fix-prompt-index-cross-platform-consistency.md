# Prompt 索引跨平台一致性修复

## 背景

`Prompt/index.json` 会记录工具包文件的大小和 SHA-256。Windows 的 Git 换行策略会将工具包中的文本文件检出为 CRLF，而 GitHub Actions 的 Ubuntu 检出为 LF，导致同一提交在本地和 CI 生成不同的索引。

## 目标

- 将 `Prompt/tools` 中的文本资源固定按 LF 检出，保证各平台读取到相同字节。
- 将 ZIP 工具包保持二进制，不参与换行转换。
- 重新生成并提交与规范化文件字节匹配的 `Prompt/index.json`。
- 保留现有 `--check` 的换行兼容比较，避免索引文件本身因换行策略产生误报。

## 非目标

- 不修改工具包源代码、ZIP 内容或工具元数据。
- 不修改 GitHub Actions 任务结构、依赖审计策略或发布流程。

## 验收方式

- Windows 当前工作树执行 `node scripts/build-prompt-index.mjs --check` 退出码为 0。
- 使用仓库规范 LF 检出的独立副本执行同一命令退出码为 0。
- `git diff --check` 通过，索引只包含由规范化文件重新生成的字节大小和 SHA-256。
- 推送后 `Security Audit / Prompt Index Consistency` 成功。
