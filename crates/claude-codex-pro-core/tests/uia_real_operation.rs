use anyhow::Result;
use claude_codex_pro_core::claude_desktop_computer_use::uia::{
    WindowsUiaBackend, FindParams, ElementType, UiElement,
};
use std::thread;
use std::time::Duration;

fn print_tree(element: &UiElement, depth: usize, max_depth: usize) {
    if depth > max_depth {
        return;
    }
    let indent = "  ".repeat(depth);
    println!("   {}- {:?} \"{}\" [{}]", indent, element.element_type, element.label, element.id);
    for child in &element.children {
        print_tree(child, depth + 1, max_depth);
    }
}

#[test]
#[ignore]
fn test_uia_real_operation() -> Result<()> {
    println!("\n=== Windows UIA 实际操作测试 ===\n");

    // 1. 创建后端
    println!("1. 初始化 UIA 后端...");
    let mut backend = WindowsUiaBackend::new()?;
    println!("   ✅ 后端创建成功\n");

    // 2. 列出当前窗口
    println!("2. 列出当前打开的窗口...");
    let windows = backend.list_windows()?;
    println!("   找到 {} 个可见窗口", windows.len());
    for (i, win) in windows.iter().take(10).enumerate() {
        println!("   [{}] {} (hwnd: 0x{:X})", i + 1, win.title, win.hwnd);
    }
    println!();

    // 3. 查找记事本
    println!("3. 查找记事本窗口...");

    // 先尝试启动记事本
    let _ = std::process::Command::new("notepad.exe").spawn();
    thread::sleep(Duration::from_secs(2));

    // 重新获取窗口列表
    let windows = backend.list_windows()?;
    let notepad = windows.iter()
        .find(|w| w.title.contains("记事本") || w.title.contains("Notepad") || w.title.contains("notepad"))
        .expect("❌ 未找到记事本！已尝试启动，请手动打开记事本后重新运行测试");

    println!("   ✅ 找到记事本: {}", notepad.title);
    println!("   hwnd: 0x{:X}\n", notepad.hwnd);

    // 4. 获取 UI 树
    println!("4. 获取记事本的 UI 树...");
    let tree = backend.get_tree(notepad.hwnd)?;
    println!("   ✅ UI 树获取成功");
    println!("   根元素: {}", tree.label);
    println!("   子元素数量: {}\n", tree.children.len());

    // 5. 查找编辑框
    println!("5. 查找编辑框...");

    // 先打印树结构，看看有什么元素
    println!("   树结构预览:");
    print_tree(&tree, 0, 3);

    let params = FindParams {
        query: None,
        element_type: Some(ElementType::Edit),
        interactive_only: false,
    };
    let edits = backend.find_elements(notepad.hwnd, &params)?;

    let edit_id = if edits.is_empty() {
        println!("   ⚠️  未找到 Edit 类型，尝试查找 Document 类型...");
        let params = FindParams {
            query: None,
            element_type: Some(ElementType::Document),
            interactive_only: false,
        };
        let docs = backend.find_elements(notepad.hwnd, &params)?;
        assert!(!docs.is_empty(), "未找到编辑框（Edit 或 Document 类型）");
        let edit = &docs[0];
        println!("   ✅ 找到文档编辑区: {} ({})", edit.label, edit.id);
        edit.id.clone()
    } else {
        let edit = &edits[0];
        println!("   ✅ 找到编辑框: {}", edit.id);
        println!("   已启用: {}", edit.enabled);
        edit.id.clone()
    };
    println!();

    // 6. 输入测试文本
    println!("6. 输入测试文本...");
    let test_text = "=== UIA 模块实际操作测试 ===\n\n";
    backend.set_text(&edit_id, test_text)?;
    println!("   ✅ 文本输入成功！\n");
    thread::sleep(Duration::from_millis(500));

    // 7. 追加 ASCII 文本
    println!("7. 追加 ASCII 文本...");
    let ascii_text = "Hello, Windows UI Automation!\n";
    backend.set_text(&edit_id, ascii_text)?;
    println!("   ✅ ASCII 文本输入成功！\n");
    thread::sleep(Duration::from_millis(500));

    // 8. 追加中文文本
    println!("8. 追加中文文本...");
    let chinese_text = "你好，世界！这是中文测试。\n";
    backend.set_text(&edit_id, chinese_text)?;
    println!("   ✅ 中文文本输入成功！\n");
    thread::sleep(Duration::from_millis(500));

    // 9. 追加 emoji
    println!("9. 追加 emoji 表情...");
    let emoji_text = "支持 emoji: 🌍 🎉 ✅ 🚀 💻\n";
    backend.set_text(&edit_id, emoji_text)?;
    println!("   ✅ Emoji 输入成功！\n");
    thread::sleep(Duration::from_millis(500));

    // 10. 追加混合文本
    println!("10. 追加混合文本...");
    let mixed_text = "Mixed: English + 中文 + 日本語 + emoji 🔥\n";
    backend.set_text(&edit_id, mixed_text)?;
    println!("   ✅ 混合文本输入成功！\n");
    thread::sleep(Duration::from_millis(500));

    // 11. 最终总结
    println!("11. 输入测试总结...");
    let summary = "\n=== 测试完成 ===\n所有文本输入功能正常工作！✅";
    backend.set_text(&edit_id, summary)?;
    println!("   ✅ 总结输入成功！\n");

    println!("=== 测试通过 ===");
    println!("请查看记事本窗口中的内容！");
    println!("等待 3 秒让你查看结果...\n");
    thread::sleep(Duration::from_secs(3));

    Ok(())
}
