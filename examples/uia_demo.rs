use anyhow::Result;
use claude_codex_pro_core::claude_desktop_computer_use::uia::{
    WindowsUiaBackend, FindCondition, ElementType, MatchType,
};
use std::thread;
use std::time::Duration;

fn main() -> Result<()> {
    println!("=== Windows UIA 模块演示 ===\n");

    // 1. 创建后端
    println!("1. 初始化 UIA 后端...");
    let mut backend = WindowsUiaBackend::new()?;
    println!("   ✅ 后端创建成功\n");

    // 2. 列出当前窗口
    println!("2. 列出当前打开的窗口...");
    let windows = backend.list_windows()?;
    println!("   找到 {} 个可见窗口：", windows.len());
    for (i, win) in windows.iter().take(10).enumerate() {
        println!("   [{}] {} (hwnd: 0x{:X})", i + 1, win.title, win.hwnd);
    }
    println!();

    // 3. 查找记事本
    println!("3. 查找记事本窗口...");
    let notepad = windows.iter().find(|w| w.title.contains("记事本") || w.title.contains("Notepad"));

    if let Some(notepad) = notepad {
        println!("   ✅ 找到记事本: {}", notepad.title);
        println!("   hwnd: 0x{:X}\n", notepad.hwnd);

        // 4. 获取 UI 树
        println!("4. 获取记事本的 UI 树...");
        let tree = backend.get_tree(notepad.hwnd)?;
        println!("   ✅ UI 树获取成功");
        println!("   根元素 ID: {}", tree.id);
        println!("   元素类型: {:?}", tree.element_type);
        println!("   标签: {}", tree.label);
        println!("   子元素数量: {}\n", tree.children.len());

        // 5. 查找编辑框
        println!("5. 查找编辑框...");
        let condition = FindCondition::ControlType(ElementType::Edit);
        match backend.find_element(&tree.id, &condition) {
            Ok(edit) => {
                println!("   ✅ 找到编辑框");
                println!("   元素 ID: {}", edit.id);
                println!("   标签: {}", edit.label);
                println!("   类型: {:?}", edit.element_type);
                println!("   已启用: {}", edit.enabled);
                println!("   可聚焦: {}\n", edit.is_keyboard_focusable);

                // 6. 设置文本
                println!("6. 向编辑框输入文本...");
                let test_text = "Hello from UIA! 你好，世界！🌍";
                println!("   输入内容: {}", test_text);

                match backend.set_text(&edit.id, test_text) {
                    Ok(_) => {
                        println!("   ✅ 文本输入成功！");
                        println!("   （请查看记事本窗口）\n");

                        // 等待用户查看
                        thread::sleep(Duration::from_secs(2));

                        // 7. 追加更多文本
                        println!("7. 追加更多文本...");
                        let more_text = "\n\n这是第二行文本。\nThis is the third line.\n测试完成！✅";
                        match backend.set_text(&edit.id, more_text) {
                            Ok(_) => println!("   ✅ 追加文本成功！\n"),
                            Err(e) => println!("   ❌ 追加失败: {}\n", e),
                        }
                    }
                    Err(e) => {
                        println!("   ❌ 文本输入失败: {}\n", e);
                    }
                }

                // 8. 查找菜单栏
                println!("8. 查找菜单栏...");
                let menu_condition = FindCondition::ControlType(ElementType::MenuBar);
                match backend.find_element(&tree.id, &menu_condition) {
                    Ok(menubar) => {
                        println!("   ✅ 找到菜单栏");
                        println!("   子菜单数量: {}", menubar.children.len());
                        for (i, menu) in menubar.children.iter().enumerate() {
                            println!("   [{}] {}", i + 1, menu.label);
                        }
                        println!();
                    }
                    Err(e) => {
                        println!("   ⚠️  未找到菜单栏: {}\n", e);
                    }
                }

                // 9. 查找按钮
                println!("9. 查找所有按钮...");
                let button_condition = FindCondition::ControlType(ElementType::Button);
                match backend.find_elements(&tree.id, &button_condition, 10) {
                    Ok(buttons) => {
                        println!("   ✅ 找到 {} 个按钮", buttons.len());
                        for (i, btn) in buttons.iter().enumerate() {
                            println!("   [{}] {} ({})", i + 1, btn.label,
                                if btn.enabled { "已启用" } else { "已禁用" });
                        }
                        println!();
                    }
                    Err(e) => {
                        println!("   ⚠️  未找到按钮: {}\n", e);
                    }
                }

            }
            Err(e) => {
                println!("   ❌ 未找到编辑框: {}\n", e);
            }
        }

    } else {
        println!("   ⚠️  未找到记事本窗口");
        println!("   提示：请先打开记事本（notepad.exe）\n");

        // 显示其他窗口作为参考
        println!("当前可见的窗口：");
        for (i, win) in windows.iter().take(5).enumerate() {
            println!("   [{}] {}", i + 1, win.title);
        }
    }

    println!("\n=== 演示完成 ===");
    println!("提示：如果看到文本输入成功，说明 UIA 模块工作正常！");

    Ok(())
}
