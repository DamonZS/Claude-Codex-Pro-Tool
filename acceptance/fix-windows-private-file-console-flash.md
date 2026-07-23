# 验收标准：修复 Windows 私有文件写入时的终端闪窗

验证对象：`spec/fix-windows-private-file-console-flash.md`

## 验收项

1. `secure_private_path` 仍通过 `icacls.exe` 收敛 Windows 私有文件 ACL，原有参数和错误处理保持不变。
2. Windows `Command` 显式导入 `CommandExt`，并使用 `windows_create_no_window()` 设置 `creation_flags`。
3. Manager 的 `windows_subsystem` 定向回归测试通过，能在启动标志被移除时失败。
4. `cargo fmt --check` 通过。
5. `cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem windows_private_file_acl_command_stays_hidden -- --nocapture` 通过。
6. `cargo test -p claude-codex-pro-core --manifest-path Cargo.toml settings::tests::atomic_write_applies_private_windows_acl -- --nocapture` 通过，并证明 ACL 行为仍有效。

## 完成证据

- 上述命令的真实退出状态和测试结果。
- 相关源码差异。

## 非目标

- 修改供应商、主题或设置的数据结构。
- 修改 ACL 授权主体或降低文件权限。
- 隐藏用户明确要求打开的外部终端。

