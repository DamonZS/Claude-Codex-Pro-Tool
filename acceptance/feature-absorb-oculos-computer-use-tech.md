# 验收标准：吸收 Oculos 电脑操作技术并应用到 CCP Computer Use

## 验证规格文档

本验收标准对应规格文档：[spec/feature-absorb-oculos-computer-use-tech.md](../spec/feature-absorb-oculos-computer-use-tech.md)

## 通过/失败标准

任务完成必须满足以下**所有**标准。任一项不满足即为未完成。

## 1. 代码结构验收

### 1.1 模块结构完整性

**验证方式**：检查文件系统

**标准**：
- ✅ 存在 `crates/claude-codex-pro-core/src/claude_desktop_computer_use/uia/` 目录
- ✅ 存在以下文件（可为空存根，但必须编译通过）：
  - `uia/mod.rs`
  - `uia/backend.rs`
  - `uia/element.rs`
  - `uia/find.rs`
  - `uia/actions.rs`
  - `uia/windows.rs`
  - `uia/screenshot.rs`
  - `uia/keys.rs`
  - `uia/registry.rs`

**命令**：
```bash
find crates/claude-codex-pro-core/src/claude_desktop_computer_use/uia -name "*.rs" | sort
```

**预期输出**：至少 9 个 `.rs` 文件

### 1.2 编译通过

**验证方式**：Cargo 编译检查

**标准**：
- ✅ `cargo check -p claude-codex-pro-core` 无错误
- ✅ `cargo build -p claude-codex-pro-core` 成功构建
- ⚠️ 允许有警告（未使用的导入、变量等），但不能有错误

**命令**：
```bash
cargo check -p claude-codex-pro-core 2>&1 | grep -E "error|warning" | tee /tmp/check.log
```

**预期**：`error` 行数为 0

## 2. 核心功能验收（阶段 1）

### 2.1 Windows UIA 初始化

**验证方式**：单元测试

**标准**：
- ✅ 能成功初始化 `IUIAutomation` COM 对象
- ✅ 能获取桌面根元素（`GetRootElement`）
- ✅ COM 初始化失败时返回明确错误（而非崩溃）

**测试代码**（`uia/backend.rs` 或 `tests.rs`）：
```rust
#[test]
fn test_uia_initialization() {
    let backend = WindowsUiaBackend::new();
    assert!(backend.is_ok(), "UIA 初始化失败：{:?}", backend.err());
}
```

**命令**：
```bash
cargo test -p claude-codex-pro-core uia_initialization -- --nocapture
```

**预期**：测试通过，输出包含 `test ... ok`

### 2.2 窗口列表

**验证方式**：集成测试 + 手动验证

**标准**：
- ✅ `list_windows()` 返回至少 1 个窗口
- ✅ 返回结果包含：PID、HWND、标题、程序名、边界矩形、是否前台
- ✅ 能识别当前前台窗口（`is_foreground: true`）

**测试代码**：
```rust
#[test]
#[ignore] // 需要真实桌面环境
fn test_list_windows() {
    let backend = WindowsUiaBackend::new().unwrap();
    let windows = backend.list_windows().unwrap();
    assert!(!windows.is_empty(), "未找到任何窗口");
    
    let has_foreground = windows.iter().any(|w| w.is_foreground);
    assert!(has_foreground, "未识别到前台窗口");
    
    println!("找到 {} 个窗口", windows.len());
    for w in windows.iter().take(5) {
        println!("  [{}] {} ({})", w.pid, w.title, w.exe_name);
    }
}
```

**命令**：
```bash
cargo test -p claude-codex-pro-core test_list_windows -- --nocapture --ignored
```

**预期**：
- 测试通过
- 输出显示至少 3 个窗口（例如：explorer.exe, claude.exe, notepad.exe）

### 2.3 UI 元素树遍历

**验证方式**：针对记事本的集成测试

**前置条件**：打开一个记事本窗口

**标准**：
- ✅ `get_ui_tree(notepad_pid)` 返回根元素
- ✅ 根元素有子元素（至少包含菜单栏、文本框）
- ✅ 每个元素有有效的 `id`（格式：`pid.runtime_id`）
- ✅ 元素的 `element_type` 正确（Window, MenuBar, Edit, Button 等）
- ✅ 文本框的 `value` 包含当前内容

**测试代码**：
```rust
#[test]
#[ignore]
fn test_ui_tree_notepad() {
    // 启动记事本
    let child = std::process::Command::new("notepad.exe").spawn().unwrap();
    let pid = child.id();
    std::thread::sleep(std::time::Duration::from_secs(2));
    
    let backend = WindowsUiaBackend::new().unwrap();
    let tree = backend.get_ui_tree(pid).unwrap();
    
    assert_eq!(tree.element_type, ElementType::Window);
    assert!(!tree.children.is_empty(), "记事本应有子元素");
    
    // 查找文本框
    let has_edit = tree.children.iter()
        .any(|c| matches!(c.element_type, ElementType::Edit));
    assert!(has_edit, "未找到文本框元素");
    
    println!("记事本 UI 树：{:#?}", tree);
    
    // 清理
    let _ = std::process::Command::new("taskkill")
        .args(&["/PID", &pid.to_string(), "/F"])
        .output();
}
```

**命令**：
```bash
cargo test -p claude-codex-pro-core test_ui_tree_notepad -- --nocapture --ignored
```

**预期**：
- 测试通过
- 输出显示记事本的 UI 树结构（至少 3 层深度）

### 2.4 元素查找

**验证方式**：针对记事本菜单的测试

**标准**：
- ✅ `find_elements(pid, query: Some("文件"), ...)` 找到"文件"菜单项
- ✅ `find_elements(pid, element_type: Some(Button), ...)` 找到所有按钮
- ✅ `interactive_only: true` 过滤掉不可交互元素
- ✅ 未找到时返回空数组（而非错误）

**测试代码**：
```rust
#[test]
#[ignore]
fn test_find_elements_notepad() {
    let child = std::process::Command::new("notepad.exe").spawn().unwrap();
    let pid = child.id();
    std::thread::sleep(std::time::Duration::from_secs(2));
    
    let backend = WindowsUiaBackend::new().unwrap();
    
    // 查找"文件"菜单项
    let results = backend.find_elements(
        pid,
        Some("文件"),
        None,
        false
    ).unwrap();
    assert!(!results.is_empty(), "未找到\"文件\"菜单项");
    println!("找到 {} 个匹配\"文件\"的元素", results.len());
    
    // 查找所有按钮
    let buttons = backend.find_elements(
        pid,
        None,
        Some(&ElementType::Button),
        false
    ).unwrap();
    println!("找到 {} 个按钮", buttons.len());
    
    // 查找不存在的元素
    let none = backend.find_elements(
        pid,
        Some("不存在的元素XYZ"),
        None,
        false
    ).unwrap();
    assert!(none.is_empty(), "不应找到不存在的元素");
    
    // 清理
    let _ = std::process::Command::new("taskkill")
        .args(&["/PID", &pid.to_string(), "/F"])
        .output();
}
```

**命令**：
```bash
cargo test -p claude-codex-pro-core test_find_elements_notepad -- --nocapture --ignored
```

**预期**：
- 测试通过
- 输出显示找到至少 1 个"文件"菜单项
- 输出显示找到至少 3 个按钮

### 2.5 元素点击

**验证方式**：点击记事本菜单并验证效果

**标准**：
- ✅ `click_element(file_menu_id)` 成功执行
- ✅ 点击后菜单展开（可通过查找"退出"菜单项验证）
- ✅ 点击不存在的元素返回 `NotFound` 错误

**测试代码**：
```rust
#[test]
#[ignore]
fn test_click_element_notepad() {
    let child = std::process::Command::new("notepad.exe").spawn().unwrap();
    let pid = child.id();
    std::thread::sleep(std::time::Duration::from_secs(2));
    
    let backend = WindowsUiaBackend::new().unwrap();
    
    // 查找"文件"菜单项
    let file_menu = backend.find_elements(pid, Some("文件"), None, false)
        .unwrap()
        .into_iter()
        .next()
        .expect("未找到\"文件\"菜单项");
    
    println!("点击菜单项：{} ({})", file_menu.name, file_menu.id);
    backend.click_element(&file_menu.id).unwrap();
    
    std::thread::sleep(std::time::Duration::from_millis(500));
    
    // 验证菜单已展开：查找"退出"菜单项
    let exit_item = backend.find_elements(pid, Some("退出"), None, false).unwrap();
    assert!(!exit_item.is_empty(), "点击后未找到\"退出\"菜单项（菜单可能未展开）");
    
    // 点击不存在的元素
    let err = backend.click_element("999.invalid_id");
    assert!(err.is_err(), "不应能点击无效元素");
    
    // 清理
    let _ = std::process::Command::new("taskkill")
        .args(&["/PID", &pid.to_string(), "/F"])
        .output();
}
```

**命令**：
```bash
cargo test -p claude-codex-pro-core test_click_element_notepad -- --nocapture --ignored
```

**预期**：
- 测试通过
- 输出显示点击成功，菜单展开

### 2.6 文本输入

**验证方式**：在记事本文本框输入并验证

**标准**：
- ✅ `set_text(edit_id, "Hello World")` 成功执行
- ✅ 文本框的 `value` 更新为"Hello World"
- ✅ 支持多行文本（包含 `\n`）

**测试代码**：
```rust
#[test]
#[ignore]
fn test_set_text_notepad() {
    let child = std::process::Command::new("notepad.exe").spawn().unwrap();
    let pid = child.id();
    std::thread::sleep(std::time::Duration::from_secs(2));
    
    let backend = WindowsUiaBackend::new().unwrap();
    
    // 查找文本框
    let tree = backend.get_ui_tree(pid).unwrap();
    let edit = tree.children.iter()
        .find(|c| matches!(c.element_type, ElementType::Edit))
        .expect("未找到文本框");
    
    println!("在文本框输入：{}", edit.id);
    backend.set_text(&edit.id, "Hello World\n第二行").unwrap();
    
    std::thread::sleep(std::time::Duration::from_millis(500));
    
    // 验证文本
    let updated_tree = backend.get_ui_tree(pid).unwrap();
    let updated_edit = updated_tree.children.iter()
        .find(|c| matches!(c.element_type, ElementType::Edit))
        .unwrap();
    
    let value = updated_edit.value.as_deref().unwrap_or("");
    assert!(value.contains("Hello World"), "文本未正确输入：{}", value);
    
    // 清理
    let _ = std::process::Command::new("taskkill")
        .args(&["/PID", &pid.to_string(), "/F"])
        .output();
}
```

**命令**：
```bash
cargo test -p claude-codex-pro-core test_set_text_notepad -- --nocapture --ignored
```

**预期**：
- 测试通过
- 输出显示文本正确输入

### 2.7 窗口聚焦

**验证方式**：启动后台记事本并聚焦

**标准**：
- ✅ `focus_window(pid)` 成功执行
- ✅ 窗口从后台移到前台（`is_foreground` 变为 `true`）
- ✅ 最小化窗口被还原并聚焦

**测试代码**：
```rust
#[test]
#[ignore]
fn test_focus_window() {
    // 启动两个记事本（第二个会在后台）
    let child1 = std::process::Command::new("notepad.exe").spawn().unwrap();
    std::thread::sleep(std::time::Duration::from_secs(1));
    let child2 = std::process::Command::new("notepad.exe").spawn().unwrap();
    let pid2 = child2.id();
    std::thread::sleep(std::time::Duration::from_secs(1));
    
    let backend = WindowsUiaBackend::new().unwrap();
    
    // 验证 pid2 在后台
    let windows = backend.list_windows().unwrap();
    let win2 = windows.iter().find(|w| w.pid == pid2).unwrap();
    println!("窗口 {} 前台状态：{}", pid2, win2.is_foreground);
    
    // 聚焦
    backend.focus_window(pid2).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(500));
    
    // 验证已聚焦
    let updated = backend.list_windows().unwrap();
    let updated_win2 = updated.iter().find(|w| w.pid == pid2).unwrap();
    assert!(updated_win2.is_foreground, "窗口未成功聚焦");
    
    // 清理
    let _ = std::process::Command::new("taskkill")
        .args(&["/PID", &child1.id().to_string(), "/F"])
        .output();
    let _ = std::process::Command::new("taskkill")
        .args(&["/PID", &pid2.to_string(), "/F"])
        .output();
}
```

**命令**：
```bash
cargo test -p claude-codex-pro-core test_focus_window -- --nocapture --ignored
```

**预期**：
- 测试通过
- 输出显示窗口成功聚焦

## 3. 高级功能验收（阶段 2）

### 3.1 复杂键盘输入

**验证方式**：使用 `{KEY}` 语法测试

**标准**：
- ✅ `send_keys(edit_id, "{CTRL+A}")` 全选文本
- ✅ `send_keys(edit_id, "{DELETE}")` 删除文本
- ✅ `send_keys(edit_id, "Test{CTRL+A}{DELETE}New")` 组合操作后文本为"New"
- ✅ `{WIN+D}` 切换到桌面（如可测试）
- ✅ `{TAB 3}` 按 Tab 键 3 次

**测试代码**：
```rust
#[test]
#[ignore]
fn test_send_keys_advanced() {
    let child = std::process::Command::new("notepad.exe").spawn().unwrap();
    let pid = child.id();
    std::thread::sleep(std::time::Duration::from_secs(2));
    
    let backend = WindowsUiaBackend::new().unwrap();
    let tree = backend.get_ui_tree(pid).unwrap();
    let edit = tree.children.iter()
        .find(|c| matches!(c.element_type, ElementType::Edit))
        .expect("未找到文本框");
    
    // 输入初始文本
    backend.set_text(&edit.id, "Initial Text").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(300));
    
    // 全选并删除
    backend.send_keys(&edit.id, "{CTRL+A}{DELETE}").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(300));
    
    // 验证已清空
    let tree2 = backend.get_ui_tree(pid).unwrap();
    let edit2 = tree2.children.iter()
        .find(|c| matches!(c.element_type, ElementType::Edit))
        .unwrap();
    let value = edit2.value.as_deref().unwrap_or("");
    assert!(value.is_empty() || value.trim().is_empty(), "文本未清空：{}", value);
    
    // 输入新文本
    backend.send_keys(&edit.id, "New Text").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(300));
    
    let tree3 = backend.get_ui_tree(pid).unwrap();
    let edit3 = tree3.children.iter()
        .find(|c| matches!(c.element_type, ElementType::Edit))
        .unwrap();
    assert!(edit3.value.as_deref().unwrap_or("").contains("New Text"));
    
    // 清理
    let _ = std::process::Command::new("taskkill")
        .args(&["/PID", &pid.to_string(), "/F"])
        .output();
}
```

**命令**：
```bash
cargo test -p claude-codex-pro-core test_send_keys_advanced -- --nocapture --ignored
```

**预期**：测试通过，文本按预期清空和输入

### 3.2 元素等待

**验证方式**：等待菜单项出现

**标准**：
- ✅ 点击菜单后，`wait_for_element(..., until: Appears, timeout: 5000)` 找到子菜单项
- ✅ 超时后返回 `Timeout` 错误（而非永久阻塞）
- ✅ `until: Disappears` 能检测元素消失

**测试代码**：
```rust
#[test]
#[ignore]
fn test_wait_for_element() {
    let child = std::process::Command::new("notepad.exe").spawn().unwrap();
    let pid = child.id();
    std::thread::sleep(std::time::Duration::from_secs(2));
    
    let backend = WindowsUiaBackend::new().unwrap();
    
    // 点击"文件"菜单
    let file_menu = backend.find_elements(pid, Some("文件"), None, false)
        .unwrap()[0].id.clone();
    backend.click_element(&file_menu).unwrap();
    
    // 等待"退出"菜单项出现
    let exit_items = backend.wait_for_element(
        pid,
        Some("退出"),
        None,
        WaitUntil::Appears,
        5000
    ).unwrap();
    assert!(!exit_items.is_empty(), "等待超时：未找到\"退出\"菜单项");
    
    // 点击退出按钮（会弹出对话框）
    backend.click_element(&exit_items[0].id).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(500));
    
    // 等待"不保存"按钮出现
    let dont_save = backend.wait_for_element(
        pid,
        Some("不保存"),
        Some(&ElementType::Button),
        WaitUntil::Appears,
        3000
    );
    
    if let Ok(buttons) = dont_save {
        println!("找到\"不保存\"按钮，点击关闭");
        backend.click_element(&buttons[0].id).unwrap();
    } else {
        // 清理
        let _ = std::process::Command::new("taskkill")
            .args(&["/PID", &pid.to_string(), "/F"])
            .output();
    }
}
```

**命令**：
```bash
cargo test -p claude-codex-pro-core test_wait_for_element -- --nocapture --ignored
```

**预期**：测试通过，找到等待的元素

### 3.3 批量操作

**验证方式**：原子化执行多步骤

**标准**：
- ✅ `batch_actions([click(文件), click(退出)])` 成功执行两步
- ✅ 中间某步失败时，后续步骤不执行（`stop_on_error: true`）
- ✅ 返回每步的执行状态（成功/失败）

**测试代码**：
```rust
#[test]
#[ignore]
fn test_batch_actions() {
    let child = std::process::Command::new("notepad.exe").spawn().unwrap();
    let pid = child.id();
    std::thread::sleep(std::time::Duration::from_secs(2));
    
    let backend = WindowsUiaBackend::new().unwrap();
    
    // 查找元素
    let file_menu = backend.find_elements(pid, Some("文件"), None, false)
        .unwrap()[0].id.clone();
    
    // 批量操作
    let actions = vec![
        BatchAction {
            element_id: file_menu.clone(),
            action: ActionType::Click,
            text: None,
            keys: None,
            value: None,
            direction: None,
        },
        // 等待菜单展开
        BatchAction {
            element_id: file_menu.clone(),
            action: ActionType::Wait,
            text: None,
            keys: None,
            value: Some(0.5), // 等待 500ms
            direction: None,
        },
    ];
    
    let results = backend.batch_actions(&actions, true, 0).unwrap();
    assert_eq!(results.len(), 2);
    assert!(results[0].success, "第一步失败：{:?}", results[0].error);
    assert!(results[1].success, "第二步失败：{:?}", results[1].error);
    
    // 清理
    let _ = std::process::Command::new("taskkill")
        .args(&["/PID", &pid.to_string(), "/F"])
        .output();
}
```

**命令**：
```bash
cargo test -p claude-codex-pro-core test_batch_actions -- --nocapture --ignored
```

**预期**：测试通过，所有步骤成功执行

### 3.4 元素高亮

**验证方式**：目视验证（或截图对比）

**标准**：
- ✅ `highlight_element(id, 2000)` 在元素周围绘制红色矩形
- ✅ 持续 2 秒后自动消失
- ✅ 不阻塞后续操作

**测试代码**：
```rust
#[test]
#[ignore]
fn test_highlight_element() {
    let child = std::process::Command::new("notepad.exe").spawn().unwrap();
    let pid = child.id();
    std::thread::sleep(std::time::Duration::from_secs(2));
    
    let backend = WindowsUiaBackend::new().unwrap();
    
    let file_menu = backend.find_elements(pid, Some("文件"), None, false)
        .unwrap()[0].id.clone();
    
    println!("高亮\"文件\"菜单项 2 秒（请目视确认）");
    let rect = backend.highlight_element(&file_menu, 2000).unwrap();
    println!("高亮区域：left={}, top={}, right={}, bottom={}", 
             rect.left, rect.top, rect.right, rect.bottom);
    
    // 高亮不应阻塞后续操作
    backend.click_element(&file_menu).unwrap();
    
    std::thread::sleep(std::time::Duration::from_secs(3));
    
    // 清理
    let _ = std::process::Command::new("taskkill")
        .args(&["/PID", &pid.to_string(), "/F"])
        .output();
}
```

**命令**：
```bash
cargo test -p claude-codex-pro-core test_highlight_element -- --nocapture --ignored
```

**预期**：
- 测试通过
- 目视看到红色矩形高亮 2 秒

## 4. 集成验收

### 4.1 端到端场景测试

**场景**：AI 自动化记事本操作

**流程**：
1. 列出窗口，找到记事本
2. 获取 UI 树
3. 查找文本框并输入"Test"
4. 使用 `{CTRL+A}{DELETE}` 清空
5. 输入新文本"Final"
6. 点击文件菜单
7. 等待"退出"菜单项出现
8. 点击退出（不保存）

**验证方式**：完整集成测试

**标准**：
- ✅ 所有步骤成功执行
- ✅ 最终文本框为空（因为未保存）
- ✅ 记事本进程已退出

**命令**：
```bash
cargo test -p claude-codex-pro-core test_end_to_end_notepad -- --nocapture --ignored
```

**预期**：测试通过，记事本按预期操作并关闭

### 4.2 MCP 工具暴露

**验证方式**：检查 `tools/list` 响应

**标准**：
- ✅ MCP 服务器的 `tools/list` 返回至少 23 个工具（8 现有 + 15 新增）
- ✅ 新增工具包含：
  - `list_windows`
  - `get_ui_tree`
  - `find_elements`
  - `click_element`
  - `set_text`
  - `send_keys_advanced`
  - `focus_window`
  - `wait_for_element`
  - `batch_actions`
  - `highlight_element`
- ✅ 每个工具有 `inputSchema` 定义

**验证代码**：
```rust
#[test]
fn test_mcp_tools_list() {
    let server = create_test_server();
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list"
    });
    
    let response = server.handle_line(&request.to_string()).unwrap();
    let resp: Value = serde_json::from_str(&response).unwrap();
    
    let tools = resp["result"]["tools"].as_array().unwrap();
    assert!(tools.len() >= 23, "工具数量不足：{}", tools.len());
    
    let tool_names: Vec<&str> = tools.iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    
    assert!(tool_names.contains(&"list_windows"));
    assert!(tool_names.contains(&"get_ui_tree"));
    assert!(tool_names.contains(&"find_elements"));
    assert!(tool_names.contains(&"click_element"));
    
    println!("工具列表：{:?}", tool_names);
}
```

**命令**：
```bash
cargo test -p claude-codex-pro-core test_mcp_tools_list -- --nocapture
```

**预期**：测试通过，工具列表完整

## 5. 性能验收

### 5.1 元素树遍历性能

**标准**：
- ✅ 记事本（<50 个元素）：遍历时间 < 500ms
- ✅ 计算器（约 100 个元素）：遍历时间 < 1000ms
- ⚠️ 大型应用（>500 个元素）：可能超时，需设置深度限制

**测试代码**：
```rust
#[test]
#[ignore]
fn test_performance_ui_tree() {
    let child = std::process::Command::new("notepad.exe").spawn().unwrap();
    let pid = child.id();
    std::thread::sleep(std::time::Duration::from_secs(2));
    
    let backend = WindowsUiaBackend::new().unwrap();
    
    let start = std::time::Instant::now();
    let tree = backend.get_ui_tree(pid).unwrap();
    let elapsed = start.elapsed();
    
    println!("遍历记事本 UI 树耗时：{:?}", elapsed);
    println!("元素总数：{}", count_elements(&tree));
    
    assert!(elapsed.as_millis() < 500, "遍历超时：{:?}", elapsed);
    
    // 清理
    let _ = std::process::Command::new("taskkill")
        .args(&["/PID", &pid.to_string(), "/F"])
        .output();
}

fn count_elements(elem: &UiElement) -> usize {
    1 + elem.children.iter().map(count_elements).sum::<usize>()
}
```

**命令**：
```bash
cargo test -p claude-codex-pro-core test_performance_ui_tree -- --nocapture --ignored
```

**预期**：耗时 < 500ms

### 5.2 元素查找性能

**标准**：
- ✅ 在 100 个元素内查找：< 200ms
- ✅ 查找不存在的元素：< 300ms

**命令**：
```bash
cargo test -p claude-codex-pro-core test_performance_find -- --nocapture --ignored
```

## 6. 稳定性验收

### 6.1 错误处理

**标准**：
- ✅ 操作不存在的元素 ID → 返回 `NotFound` 错误（而非崩溃）
- ✅ 操作已关闭窗口的元素 → 返回 `NotFound` 或 `Gone` 错误
- ✅ COM 初始化失败 → 返回明确错误信息
- ✅ 等待超时 → 返回 `Timeout` 错误
- ✅ 无效的 `{KEY}` 语法 → 返回 `InvalidInput` 错误

**命令**：
```bash
cargo test -p claude-codex-pro-core test_error_handling -- --nocapture
```

### 6.2 连续操作稳定性

**标准**：
- ✅ 连续 10 次 `list_windows` 无崩溃
- ✅ 连续 10 次 `get_ui_tree` 无崩溃
- ✅ 连续 10 次 `click_element` + `wait_for_element` 无崩溃

**命令**：
```bash
cargo test -p claude-codex-pro-core test_stability -- --nocapture --ignored
```

## 7. 文档验收

### 7.1 用户文档

**标准**：
- ✅ 存在 `docs/computer-use-uia-guide.md`
- ✅ 包含至少 5 个 AI 可用的示例 prompt
- ✅ 说明新增工具的用途和限制
- ✅ 说明已知限制（不能操作管理员窗口、UAC 提示）

**命令**：
```bash
test -f docs/computer-use-uia-guide.md && echo "文档存在" || echo "文档缺失"
```

### 7.2 架构文档

**标准**：
- ✅ `docs/architecture.md` 包含 UIA 模块说明
- ✅ 说明 `Backend` 和 `UiaBackend` 的分层关系
- ✅ 说明元素 ID 的格式和稳定性

**命令**：
```bash
grep -i "uia\|ui automation" docs/architecture.md
```

## 8. 回归验收

### 8.1 现有功能无回归

**标准**：
- ✅ 现有 8 个工具仍正常工作（`screenshot`, `click`, `move_mouse` 等）
- ✅ 急停机制不受影响（鼠标左上角触发）
- ✅ MCP 服务器仍能通过 stdin/stdout 通信
- ✅ Claude Desktop 注册流程不受影响

**命令**：
```bash
cargo test -p claude-codex-pro-core existing_tools -- --nocapture
```

### 8.2 构建与发布

**标准**：
- ✅ `cargo build --release -p claude-codex-pro-core` 成功
- ✅ 生成的二进制文件体积增长 < 2MB（相比现有版本）
- ✅ Windows 7/10/11 兼容性测试通过

**命令**：
```bash
cargo build --release -p claude-codex-pro-core
ls -lh target/release/claude-codex-pro-core.exe
```

## 9. 最终交付检查清单

在宣称任务完成前，必须确认以下所有项：

### 9.1 代码交付

- [ ] `crates/claude-codex-pro-core/src/claude_desktop_computer_use/uia/` 模块完整
- [ ] 所有新增工具有对应的 Rust 函数实现
- [ ] 至少 15 个单元测试（核心逻辑）
- [ ] 至少 5 个集成测试（真实应用场景）
- [ ] `cargo test -p claude-codex-pro-core` 通过（允许 `#[ignore]` 测试需手动运行）
- [ ] `cargo clippy -p claude-codex-pro-core` 无警告（或已记录豁免原因）

### 9.2 功能交付

- [ ] 所有阶段 1 功能验收通过（2.1-2.7）
- [ ] 至少 50% 的阶段 2 功能验收通过（3.1-3.4）
- [ ] 端到端场景测试通过（4.1）
- [ ] MCP 工具暴露验证通过（4.2）

### 9.3 性能与稳定性

- [ ] 性能验收通过（5.1-5.2）
- [ ] 错误处理验证通过（6.1）
- [ ] 连续操作稳定性通过（6.2）

### 9.4 文档交付

- [ ] 用户文档存在且完整（7.1）
- [ ] 架构文档更新（7.2）

### 9.5 回归验证

- [ ] 现有功能无回归（8.1）
- [ ] 构建与发布验证通过（8.2）

## 10. 验收失败场景

以下情况视为验收**失败**：

1. ❌ 任何阶段 1 功能验收未通过（2.1-2.7）
2. ❌ `cargo build -p claude-codex-pro-core` 失败
3. ❌ 在真实 Windows 环境连续操作 10 次出现崩溃
4. ❌ 现有 8 个工具任一功能回归
5. ❌ 用户文档缺失或不完整
6. ❌ 元素树遍历超时（记事本 > 1 秒）
7. ❌ 所有测试均标记为 `#[ignore]` 且无手动验证证据

## 11. 证据要求

任务完成时必须提供以下证据：

1. **编译日志**：`cargo build --release -p claude-codex-pro-core` 的完整输出
2. **测试日志**：至少 5 个关键测试的 `-- --nocapture` 输出
3. **截图证据**：
   - 元素高亮测试的截图（红框）
   - 端到端场景测试的最终状态截图
4. **工具列表**：MCP `tools/list` 的完整 JSON 输出
5. **性能数据**：元素树遍历和查找的耗时统计

## 12. 可选加分项

以下项非必需，但能显著提升交付质量：

- ✨ 支持多显示器截图（阶段 3）
- ✨ 实现 `screenshot_element` 工具
- ✨ macOS 平台支持（需单独评估）
- ✨ 实现元素缓存机制（性能优化）
- ✨ 提供 AI prompt 示例库（>10 个实用场景）
- ✨ 录制演示视频（展示新功能）

---

**最终裁判**：验收标准 1-9 全部通过，且提供完整证据，视为任务完成。
