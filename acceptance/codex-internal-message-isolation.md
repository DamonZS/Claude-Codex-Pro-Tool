# Codex 内部通讯隔离验收

对应规格：`spec/codex-internal-message-isolation.md`

## 通过标准

- 注入脚本包含内部消息标记、最小节点隐藏逻辑和诊断属性。
- 首次加载会执行隔离，MutationObserver 触发的扫描也会执行隔离。
- Rust 注入脚本回归测试通过，JavaScript 语法检查通过。
- 不修改线程存储、后端消息接口或普通会话选择器逻辑。

## 验证方式

```powershell
node --check assets/inject/renderer-inject.js
cargo test -p claude-codex-pro-core --test cdp_bridge injection_script_hides_codex_internal_transport_messages_from_conversation -- --nocapture
```

## 非目标检查

Codex 桌面端传输层仍可能生成内部帧；本补丁只保证 CCP 注入后的用户视图隔离。
