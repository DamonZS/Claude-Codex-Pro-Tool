# 验收标准：Claude 汉化提权可执行文件路径验证

验证对象：`spec/claude-zh-patch-executable-path-validation.md`

## 通过标准

1. `validate_executable_path` 使用 `ProgramFiles`、`ProgramW6432`、`ProgramFiles(x86)`、`WINDIR`/`SystemRoot` 和用户数据目录，不再只依赖固定 `C:\` 前缀。
2. 路径规范化后，非 C 盘、大小写不同以及 `\\?\` / `\\.\` 前缀路径可以匹配对应允许目录。
3. 目录边界严格检查，同名前缀旁系目录不能通过。
4. 新增单元测试通过，核心 Claude 汉化测试通过。

## 验证方式

- `cargo test -p claude-codex-pro-manager executable_allowlist_accepts_windows_extended_paths_and_rejects_sibling_prefixes -- --nocapture`
- `cargo test -j1 -p claude-codex-pro-core --lib claude_zh_patch -- --nocapture`
- `rustfmt --edition 2024 --check apps/claude-codex-pro-manager/src-tauri/src/commands.rs`

## 非目标

- 本验收不要求实际修改用户电脑中的 Claude 文件，也不覆盖 UAC 交互本身。
