# Windows 系统提示词权限恢复验收标准

对应规格：`spec/fix-system-prompt-permission-recovery.md`

## 通过标准

1. 五个内置模板仍由二进制内嵌资源提供，不依赖安装目录中的可读 Markdown 文件。
2. Windows 私有原子写生成的文件 DACL 不继承父目录 ACE，并显式包含当前用户 SID 与 `SYSTEM` 的 FullControl ACE；`OWNER RIGHTS` 不能作为当前用户授权的替代。
3. raw error 5 被识别为权限恢复场景；主 `state.json` 被字节区间锁锁定、实际读取返回 raw error 33 时，打开存储和列出提示词成功，返回五个内置模板并标记恢复状态。
4. 首次恢复会创建稳定的同目录恢复状态文件；再次打开存储继续使用该文件，保存和启停操作不会回到不可读的主状态。
5. 回归测试证明主状态文件在恢复前后内容不变，修复不删除、覆盖、重命名或截断旧数据。
6. 主状态文件为可读但损坏的 JSON 时仍明确失败，不自动创建恢复状态掩盖数据损坏。
7. 页面在恢复状态下继续显示和允许操作模板卡片，并展示非阻塞提示，明确原文件未覆盖、旧自定义提示词当前不可见且触发原因可能是权限或文件占用；普通状态不显示该提示。
8. Unix 私有目录和文件权限、系统提示词现有启用/停用/外部修改保护行为不回归。
9. 供应商、路由、登录、模型映射、Codex 注入、Claude 配置和内置提示词正文未被修改。
10. 旧状态不可读但 Codex 仍加载 CCP 托管文件时，页面标记孤立托管状态；重新启用后再停用会移除 `model_instructions_file`，不会把 CCP 自身托管路径恢复回去。
11. Windows 私有文件安全步骤在截断前完成；强制安全步骤失败时，已有文件字节保持不变。

## 验证方式

```powershell
cargo test -p claude-codex-pro-core system_prompt -- --nocapture
cargo test -p claude-codex-pro-core settings::tests::atomic_write_applies_private_windows_acl -- --nocapture
cargo test -p claude-codex-pro-core --lib -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo fmt --check
cargo build --release
```

## 完成证据

- Windows ACL 测试输出证明当前用户 SID 和 `S-1-5-18` 都是显式 FullControl ACE，且没有继承 ACE。
- 权限拒绝回归测试输出证明五个内置模板可列出、恢复状态可复用、主文件字节未变化。
- JSON 损坏测试、现有系统提示词测试、前端类型检查和构建结果。
- `target/release/claude-codex-pro.exe` 存在且为本次源码构建产物。

## 失败标准

- 遇到 `PermissionDenied` 仍返回整页加载失败或只返回空列表。
- 为恢复默认模板而删除、改写或接管无法读取的主状态文件。
- 每次命令重新选择主状态，导致保存、启用或停用状态丢失。
- ACL 仍只含 `OWNER RIGHTS + SYSTEM`，或通过放开到 Users/Everyone 规避问题。
- JSON 损坏被当作权限问题静默重置。

## 非目标

- 不要求自动恢复无法读取的旧自定义提示词内容。
- 不要求绕过企业组策略、杀毒软件或持续锁定整个用户数据目录的外部程序。
