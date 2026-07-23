# 修复 Windows 私有文件写入时的终端闪窗

## 背景

管理工具保存供应商设置和应用 Codex 主题时会写入一个或多个私有文件。Windows 权限收敛通过 `icacls.exe` 完成；管理工具是 GUI 应用，若子进程未使用无窗口启动标志，每次权限收敛都会短暂显示终端窗口。主题事务会连续写入多份文件，因此表现为大量终端窗口闪出后自动关闭。

## 目标

- Windows 上执行私有文件 ACL 收敛时不得创建可见终端窗口。
- 保持现有 `icacls.exe` 参数、ACL、错误处理和跨平台行为不变。
- 覆盖供应商设置保存和 Codex 主题写入共同使用的 `settings::atomic_write` 路径。

## 用户视角描述

用户保存或切换供应商、应用或恢复 Codex 主题时，操作正常完成，期间不再出现自动关闭的终端窗口。

## 功能与技术要求

- `secure_private_path` 在 Windows 上启动 `icacls.exe` 时设置 `CREATE_NO_WINDOW`。
- 使用项目已有的 `windows_create_no_window` 常量，避免重复硬编码 Windows 标志值。
- 非 Windows 平台实现不变。
- 不改变文件位置、文件内容、ACL 授权对象或失败返回语义。

## 交付范围

- Windows 私有文件权限命令修复。
- 防止该启动标志被移除的回归测试。
- 本规格与对应验收文档。

