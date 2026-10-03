# UIA 键盘输入功能

## 概述

为 Windows UIA 后端添加了键盘输入功能，提供了当 ValuePattern 不可用时的文本输入回退方案。

## 新增模块

### `keyboard.rs`

实现了键盘输入的核心功能：

- **Unicode 文本输入**：`send_unicode_text()` - 支持任意 Unicode 字符（包括中文、emoji 等）
- **虚拟键输入**：`send_virtual_key()` - 发送特殊键（Enter、Delete 等）
- **组合键**：`send_key_combination()` - 支持 Ctrl+A 等快捷键
- **文本清除**：`clear_text_field()` - 使用 Ctrl+A + Delete 清空文本框
- **延迟配置**：`KeyboardTiming` - 可配置按键延迟

## 修改的模块

### `actions.rs`

更新了 `set_text()` 方法，添加了智能回退逻辑：

1. **首选 ValuePattern**：如果元素支持 ValuePattern 且非只读，使用该模式
2. **键盘输入回退**：如果 ValuePattern 失败或不可用，自动切换到键盘输入
3. **自动清除**：键盘输入前会先清空现有内容，确保文本替换而非追加

## 使用示例

```rust
use claude_codex_pro_core::claude_desktop_computer_use::uia::WindowsUiaBackend;

let backend = WindowsUiaBackend::new()?;

// 自动选择最佳输入方式
backend.set_text(&element_id, "Hello 世界")?;

// 低层 API - 直接使用键盘
use claude_codex_pro_core::claude_desktop_computer_use::uia::keyboard::{
    send_unicode_text, KeyboardTiming
};

let timing = KeyboardTiming::default();
send_unicode_text("Hello World", &timing)?;
```

## 测试

### 单元测试

- `keyboard.rs` 中包含基础功能测试（标记为 `#[ignore]`，需要活动窗口）

### 集成测试

- `tests/uia_keyboard_integration.rs` - 完整的记事本集成测试
- 测试场景：
  - ASCII 文本输入
  - Unicode（中文）文本输入
  - 混合文本输入
  - 文本替换功能

### 运行测试

```bash
# 运行所有 UIA 测试（跳过 ignore 标记的）
cargo test --package claude-codex-pro-core --lib claude_desktop_computer_use::uia

# 运行集成测试（需要打开记事本）
cargo test --package claude-codex-pro-core --test uia_keyboard_integration -- --ignored
```

## 配置选项

```rust
pub struct KeyboardTiming {
    /// 按键之间的延迟（毫秒）
    pub key_delay_ms: u64,
    /// 按下和释放之间的延迟（毫秒）
    pub press_release_delay_ms: u64,
}

// 默认配置
impl Default for KeyboardTiming {
    fn default() -> Self {
        Self {
            key_delay_ms: 10,
            press_release_delay_ms: 5,
        }
    }
}
```

## 兼容性

- **平台**：仅支持 Windows
- **API**：使用 `SendInput` API，兼容 Windows 7+
- **Unicode**：完整支持 Unicode 字符输入
- **焦点管理**：自动处理元素聚焦

## 注意事项

1. **焦点要求**：键盘输入需要目标元素获得焦点
2. **延迟调整**：对于响应较慢的应用，可能需要增加延迟值
3. **测试环境**：手动测试需要在 GUI 环境中运行，且不能有其他输入干扰
4. **只读元素**：ValuePattern 会检查只读状态，键盘输入不会
5. **国际化**：Unicode 输入使用 `KEYEVENTF_UNICODE` 标志，与键盘布局无关

## 性能考虑

- **ValuePattern 优先**：因为它是原子操作，速度更快
- **键盘输入延迟**：默认每个字符 10ms，对于长文本可能需要优化
- **批量输入**：`send_unicode_text` 已优化为批量发送

## 未来改进

1. **更多特殊键**：扩展虚拟键支持（方向键、功能键等）
2. **高级组合键**：支持三键组合（Ctrl+Shift+A）
3. **输入验证**：添加输入后的验证机制
4. **自适应延迟**：根据应用响应动态调整延迟
5. **剪贴板方案**：对于大段文本，使用剪贴板 + Ctrl+V 可能更高效
