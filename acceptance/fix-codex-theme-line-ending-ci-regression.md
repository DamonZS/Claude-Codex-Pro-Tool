# 验收标准：Codex 主题换行符 CI 回归修复

验证对象：`spec/fix-codex-theme-line-ending-ci-regression.md`

## 验收项

1. CSS 载荷使用确定的 LF 规范形式。
   - 通过标准：主题校验和活动主题 payload 生成路径均将 CRLF 与独立 CR 规范为 LF。
   - 证据：`codex_theme.rs` 源码检查及定向回归测试输出。

2. 不同换行符输入产生相同 payload。
   - 通过标准：测试显式构造 LF 与混合 CRLF/CR 的相同主题，经真实导入、应用和 payload 生成路径后，两份 `CodexThemePayload` 完全相等，CSS 中不含 CR。
   - 证据：`cargo test -p claude-codex-pro-core --lib codex_theme::tests -- --test-threads=1` 成功。

3. 仓库主题目录和归档在 Windows 上保持一致。
   - 通过标准：`repository_theme_directories_and_archives_compile_to_the_same_payload` 成功，不再因 Git 检出的换行符不同失败。
   - 证据：同一主题定向测试命令输出。

4. 二进制主题资源行为保持不变。
   - 通过标准：规范化函数只接收已解码 CSS 字符串；图片等资源仍通过字节读取、校验和 Data URI 编译。
   - 证据：受影响文件 diff 与现有主题测试结果。

5. Rust 格式与工作区回归通过。
   - 通过标准：`cargo fmt --check` 和 `cargo test --workspace` 均成功。
   - 证据：命令退出码和测试摘要。

## 完成证据

- 三个修改文件的 diff。
- 格式检查、主题定向测试和工作区测试的命令、退出码与摘要。
- Git 工作区状态，确认没有主题资源、工作流或发布脚本改动。

## 非目标

- 不验证发布产物上传、签名或版本号策略。
- 不重写主题 ZIP 或工作区 CSS 文件。
- 不修改前端视觉和交互。
