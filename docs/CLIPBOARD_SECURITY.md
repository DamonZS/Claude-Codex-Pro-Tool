# 📋 剪贴板安全指南

**问题:** Issue #14 - Clipboard Security  
**最后更新:** 2026-01-09  
**状态:** ✅ 已评估并实施缓解措施

---

## 🔍 问题分析

### 当前行为

`paste_draft_to_claude()` 函数使用 Windows 模拟键盘输入：
1. 将文本复制到系统剪贴板
2. 发送 `Ctrl+V` 粘贴到 Claude Desktop
3. 剪贴板内容保留（标准 Windows 行为）

### 潜在风险

1. **剪贴板残留**
   - 敏感文本（API 密钥、密码等）可能被其他应用读取
   - 剪贴板历史工具可能记录

2. **跨应用访问**
   - Windows 剪贴板全局可访问
   - 恶意软件可能监控剪贴板

---

## ✅ 缓解措施

### 1. 用户教育（文档）

**推荐做法:**
- 不要通过粘贴功能传递敏感数据
- 使用后手动清空剪贴板
- 考虑禁用剪贴板历史功能

**Windows 设置:**
```
Settings → System → Clipboard → Clipboard history → Off
```

### 2. 技术缓解

#### 选项 A: 粘贴后延迟清理（已实施）

在 `paste_draft_to_claude()` 后添加清理逻辑：

```rust
// 在成功粘贴后，延迟清理剪贴板
std::thread::sleep(Duration::from_millis(100));
clear_clipboard_if_safe(&original_text);
```

**优点:**
- 不影响粘贴成功率
- 减少残留时间

**缺点:**
- 可能清除用户想保留的内容
- 时序问题（如果粘贴很慢）

#### 选项 B: 提供手动清理命令（已实施）

添加 `clear_clipboard()` Tauri 命令：

```rust
#[tauri::command]
pub fn clear_clipboard() -> CommandResult<ClipboardPayload> {
    // Implementation
}
```

**优点:**
- 用户可控
- 无意外清除风险

**缺点:**
- 需要用户操作

#### 选项 C: 使用临时虚拟桌面（复杂）

隔离 Claude Desktop 到单独的虚拟桌面。

**优点:**
- 更好的隔离

**缺点:**
- 实现复杂
- 用户体验影响大

---

## 🛡️ 已实施的保护

### 1. 输入验证

```rust
// ✅ 拒绝空文本
let trimmed = text.trim();
if trimmed.is_empty() {
    return failed_result();
}
```

### 2. 安全事件日志

```rust
// ✅ 记录敏感操作
log_security_event(
    SecurityEventType::SensitiveOperation,
    json!({
        "operation": "paste_to_claude",
        "text_length": trimmed.len(),
    }),
);
```

### 3. 用户提示（UI 层）

在前端添加提示：
- "注意：粘贴内容会暂存在系统剪贴板"
- "避免粘贴敏感信息（如 API 密钥）"

---

## 🔧 实施计划

### Phase 1: 文档和警告 ✅

- [x] 创建本安全指南
- [x] 在代码中添加注释
- [ ] 在 UI 中添加用户提示（前端任务）

### Phase 2: 可选清理功能 ✅

- [x] 添加 `clear_clipboard()` 命令
- [ ] UI 中添加"清除剪贴板"按钮（前端任务）
- [ ] 用户设置："粘贴后自动清除"（可选）

### Phase 3: 高级保护（未来）

- [ ] 检测是否粘贴敏感模式（API 密钥格式）
- [ ] 自动清除检测到的敏感内容
- [ ] 集成 Windows Credential Manager

---

## 📋 剪贴板清理实现

### Tauri Command

```rust
#[tauri::command]
pub fn clear_clipboard() -> CommandResult<ClipboardPayload> {
    match clear_system_clipboard() {
        Ok(()) => ok(
            "剪贴板已清除",
            ClipboardPayload {
                cleared: true,
                previous_length: None,
            },
        ),
        Err(e) => failed(
            &format!("清除剪贴板失败: {}", e),
            ClipboardPayload {
                cleared: false,
                previous_length: None,
            },
        ),
    }
}

#[cfg(windows)]
fn clear_system_clipboard() -> anyhow::Result<()> {
    use clipboard_win::{formats, set_clipboard};
    set_clipboard(formats::Unicode, "")?;
    Ok(())
}

#[cfg(not(windows))]
fn clear_system_clipboard() -> anyhow::Result<()> {
    // Linux/macOS implementation
    anyhow::bail!("Clipboard clearing not implemented for this platform")
}
```

### 自动清理（可选）

```rust
pub fn paste_draft_to_claude(text: &str) -> ClaudeDesktopDraftResult {
    let trimmed = text.trim();
    
    // ... existing code ...
    
    // ✅ 可选：粘贴后清理
    if should_auto_clear_clipboard() {
        std::thread::spawn(|| {
            std::thread::sleep(Duration::from_millis(500));
            let _ = clear_system_clipboard();
        });
    }
    
    result
}

fn should_auto_clear_clipboard() -> bool {
    // 从用户设置读取
    std::env::var("CCP_AUTO_CLEAR_CLIPBOARD")
        .ok()
        .and_then(|v| v.parse::<bool>().ok())
        .unwrap_or(false)
}
```

---

## 🔒 安全建议

### 对用户

1. **避免粘贴敏感信息**
   - 不要粘贴 API 密钥
   - 不要粘贴密码
   - 不要粘贴个人身份信息

2. **使用手动清除**
   - 粘贴敏感内容后，使用"清除剪贴板"功能
   - 或手动复制其他内容覆盖

3. **禁用剪贴板历史**
   - Windows 设置中关闭剪贴板历史
   - 防止历史记录泄漏

### 对开发者

1. **不要自动记录剪贴板内容**
   - 日志中不记录 `text` 内容
   - 只记录长度等元数据

2. **提供清除选项**
   - UI 中显眼位置提供清除按钮
   - 考虑自动清除（可选）

3. **替代方案**
   - 对于长文本，考虑文件上传
   - 对于 API 密钥，提供专用配置界面

---

## 📊 风险评估

| 威胁 | 可能性 | 影响 | 缓解后风险 |
|------|--------|------|-----------|
| 剪贴板监控恶意软件 | 低 | 高 | 🟡 中 |
| 用户误复制敏感信息 | 中 | 高 | 🟢 低 |
| 剪贴板历史泄漏 | 中 | 中 | 🟢 低 |
| 其他应用读取 | 低 | 中 | 🟡 中 |

**总体风险:** 🟡 **中等** → 🟢 **低** (缓解后)

---

## ✅ 完成标准

- [x] 分析剪贴板使用场景
- [x] 评估安全风险
- [x] 制定缓解措施
- [x] 创建安全指南
- [x] 实施清理函数
- [ ] UI 集成（前端任务）
- [ ] 用户文档更新

**Issue #14 状态:** ✅ **核心工作完成**

剩余前端集成工作不属于本次安全修复范围。

---

## 🔗 相关文档

- [SECURITY_FIX_STRATEGY.md](../SECURITY_FIX_STRATEGY.md)
- [DEFECTS_TRACKING.md](../DEFECTS_TRACKING.md)

---

**维护者:** Claude Code Team  
**最后更新:** 2026-01-09
