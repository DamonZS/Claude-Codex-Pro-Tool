# Windows UI Automation (UIA) 模块

Windows UI Automation 后端实现，为 Claude Desktop Computer Use 提供 Windows 平台的 UI 自动化能力。

## 📁 模块结构

```
uia/
├── mod.rs          # 模块导出
├── types.rs        # 类型定义（ElementType, FindCondition, etc）
├── backend.rs      # UIA 后端核心（WindowsUiaBackend）
├── find.rs         # 查找系统
├── actions.rs      # 元素操作（点击、输入、切换等）
├── keyboard.rs     # 键盘输入功能
└── tests.rs        # 单元测试
```

## ✨ 核心功能

### 1. 元素发现
- **树遍历**: 获取完整的 UI 元素树
- **条件查找**: 支持多条件组合（AND/OR）
- **智能缓存**: 基于 RuntimeId 的哈希缓存

### 2. 元素交互
- **点击**: InvokePattern
- **文本输入**: ValuePattern + 键盘回退
- **切换**: TogglePattern（复选框等）
- **展开/折叠**: ExpandCollapsePattern
- **选择**: SelectionItemPattern
- **范围值**: RangeValuePattern

### 3. 键盘输入
- **Unicode 支持**: 完整的 Unicode 字符输入（中文、emoji 等）
- **虚拟键**: Enter, Delete, Ctrl 等特殊键
- **组合键**: Ctrl+A, Ctrl+C 等快捷键
- **智能回退**: ValuePattern 失败时自动使用键盘输入

## 🚀 快速开始

```rust
use claude_codex_pro_core::claude_desktop_computer_use::uia::{
    WindowsUiaBackend, FindCondition, ElementType
};

// 创建后端
let mut backend = WindowsUiaBackend::new()?;

// 列出窗口
let windows = backend.list_windows()?;
let notepad = windows.iter()
    .find(|w| w.title.contains("Notepad"))
    .unwrap();

// 获取 UI 树
let tree = backend.get_tree(notepad.hwnd)?;

// 查找元素
let condition = FindCondition::ControlType(ElementType::Edit);
let edit = backend.find_element(&tree.id, &condition)?;

// 设置文本（自动选择最佳方式）
backend.set_text(&edit.id, "Hello, World!")?;
```

## 🧪 测试

```bash
# 运行所有自动化测试
cargo test --package claude-codex-pro-core --lib claude_desktop_computer_use::uia

# 运行手动测试（需要打开记事本）
cargo test --package claude-codex-pro-core --lib claude_desktop_computer_use::uia -- --ignored

# 运行键盘输入集成测试
cargo test --package claude-codex-pro-core --test uia_keyboard_integration -- --ignored
```

## 📚 文档

- **[UIA_KEYBOARD_INPUT.md](../../../docs/UIA_KEYBOARD_INPUT.md)** - 键盘输入功能详解
- **[UIA_PROGRESS_REPORT.md](../../../docs/UIA_PROGRESS_REPORT.md)** - 开发进度报告
- **[UIA_IMPLEMENTATION_COMPLETE.md](../../../docs/UIA_IMPLEMENTATION_COMPLETE.md)** - 最终交付报告

## 🎯 架构设计

### 缓存策略
使用 RuntimeId 哈希作为缓存键，保证：
- **稳定性**: 元素在应用生命周期内 ID 不变
- **性能**: O(1) 查找速度
- **内存效率**: 只缓存必要的元素引用

### 文本输入回退
```
set_text()
  ├─➊ 尝试 ValuePattern（快速、原子）
  └─➋ 失败则键盘输入（通用、可靠）
      ├─ 聚焦元素
      ├─ Ctrl+A + Delete 清空
      └─ 逐字符输入
```

### COM 生命周期
```rust
thread_local! {
    static COM_APARTMENT: ComApartment = ComApartment::enter();
}
```
每个线程独立的 COM apartment，使用 RAII 模式管理生命周期。

## ⚙️ 配置选项

### 键盘输入延迟
```rust
use claude_codex_pro_core::claude_desktop_computer_use::uia::keyboard::KeyboardTiming;

let timing = KeyboardTiming {
    key_delay_ms: 20,              // 按键之间延迟
    press_release_delay_ms: 10,    // 按下和释放延迟
};
```

### 树遍历深度
```rust
// backend.rs 中定义
const MAX_TREE_DEPTH: usize = 48;
```

## 🔍 查找示例

### 简单查找
```rust
// 按类型
let condition = FindCondition::ControlType(ElementType::Button);

// 按名称（精确匹配）
let condition = FindCondition::Name {
    value: "OK".to_string(),
    match_type: MatchType::Exact,
};
```

### 组合查找
```rust
// AND 条件
let condition = FindCondition::And(vec![
    FindCondition::Name {
        value: "Save".to_string(),
        match_type: MatchType::Contains,
    },
    FindCondition::ControlType(ElementType::Button),
]);

// OR 条件
let condition = FindCondition::Or(vec![
    FindCondition::ControlType(ElementType::Button),
    FindCondition::ControlType(ElementType::MenuItem),
]);
```

### 正则匹配
```rust
let condition = FindCondition::Name {
    value: r"^(OK|Cancel)$".to_string(),
    match_type: MatchType::Regex,
};
```

## 🐛 调试技巧

### 打印 UI 树
```rust
let tree = backend.get_tree(hwnd)?;
println!("{:#?}", tree);
```

### 查看元素信息
```rust
let element = backend.find_element(&root_id, &condition)?;
println!("Element: {}", element.label);
println!("Type: {:?}", element.element_type);
println!("Enabled: {}", element.enabled);
```

### 手动测试
```rust
#[test]
#[ignore]  // 标记为手动测试
fn test_my_feature() {
    // 测试代码
}
```

运行: `cargo test test_my_feature -- --ignored`

## 📊 性能基准

### 典型性能（记事本窗口）
- **树获取**: ~50ms（50 个元素）
- **元素查找**: ~10ms（首次）/ <1ms（缓存）
- **点击操作**: ~50ms
- **文本输入**: ~10ms（ValuePattern）/ ~100-500ms（键盘输入，取决于文本长度）

### 优化建议
1. **缓存复用**: 尽量复用元素 ID，避免重复查找
2. **条件优化**: 优先使用类型和 AutomationId 查找
3. **延迟调整**: 慢速应用需增加键盘延迟

## 🔐 安全考虑

### Unsafe 代码
所有 unsafe 代码都在 COM 互操作边界：
- ✅ 已仔细审查内存安全
- ✅ 遵循 windows-rs 安全指南
- ✅ 错误传播正确处理

### 线程安全
- ✅ `WindowsUiaBackend` 实现 `Send + Sync`
- ✅ COM 初始化使用 `thread_local`
- ✅ 元素缓存使用 `SafeElement` 包装

## 📝 注意事项

1. **焦点管理**: 键盘输入需要目标元素获得焦点
2. **延迟调整**: 不同应用响应速度不同，可能需要调整延迟
3. **只读元素**: ValuePattern 会检查只读状态，键盘输入不会
4. **Windows 版本**: 支持 Windows 7+，推荐 Windows 10+

## 🤝 贡献指南

### 添加新功能
1. 在对应模块中添加实现
2. 添加单元测试
3. 更新文档和示例
4. 运行 `cargo test` 确保测试通过

### 代码风格
- 遵循 Rust 标准风格
- 使用 `cargo fmt` 格式化
- 使用 `cargo clippy` 检查
- 添加详细的文档注释

## 📄 许可

与主项目相同的许可证。

## 🙏 致谢

基于 Microsoft UI Automation API 和 windows-rs 项目。
