# CI Windows ACL 测试环境验收

对应规格：`spec/ci-windows-acl-test-environment.md`

## 通过标准

1. 模块可加载时，ACL 测试执行原有 DACL 断言。
2. 模块不可加载时，依赖 `Get-Acl` 的测试跳过并记录原因，不产生 panic。
3. `icacls.exe` 失败、ACL 写入失败或其他测试失败仍使 `cargo test --workspace` 失败。
4. 生产 ACL 参数、授权主体和错误传播语义没有变化。

## 验证方式

- `cargo fmt --all --check`
- `cargo test -p claude-codex-pro-core --lib`
- GitHub Actions `PR build artifacts` Windows Rust tests 成功。

## 非目标

- 不安装或替换 Runner 的 PowerShell 模块。
- 不降低实际文件 ACL 的权限要求。
