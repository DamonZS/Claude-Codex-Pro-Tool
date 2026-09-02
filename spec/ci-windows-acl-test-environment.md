# CI Windows ACL 测试环境兼容

## 背景

Windows Runner 可能能解析 `Get-Acl`，但无法加载 `Microsoft.PowerShell.Security` 模块。此时 ACL 生产逻辑仍可执行，只有通过 PowerShell 读取 DACL 的断言无法取得证据，导致整套 CI 被环境缺陷阻断。

## 目标

- ACL 断言测试在执行前显式探测模块是否可加载。
- 模块不可用时仅跳过依赖 PowerShell DACL 读取的断言并输出原因。
- 模块可用时保持显式 SID、SYSTEM、非继承和 OWNER RIGHTS 断言不变。
- 不修改生产 ACL 写入、权限主体或失败传播语义。

## 技术约束

- 探测仅存在于 `#[cfg(test)]` Windows 测试模块。
- 使用无窗口、非交互 PowerShell；不写入凭据或用户数据。
- 其他测试和非环境 ACL 错误仍必须使 CI 失败。

## 交付范围

- `claude_zh_patch` 与 `settings` 的 Windows ACL 读取测试辅助函数。
- 对应单元测试和 CI 运行验证。
