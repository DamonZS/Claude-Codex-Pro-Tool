#[cfg(all(test, windows))]
mod paint_doraemon_tests {
    use claude_codex_pro_core::claude_desktop_computer_use::{Backend, NativeBackend};
    use claude_codex_pro_core::claude_desktop_computer_use::uia::WindowsUiaBackend;
    use std::thread;
    use std::time::Duration;
    use std::process::Command;

    #[test]
    #[ignore]
    fn test_draw_doraemon() {
        println!("🎨 开始在画图应用中绘制哆啦A梦...");

        // 启动画图应用
        println!("正在启动画图应用...");
        Command::new("mspaint.exe")
            .spawn()
            .expect("无法启动画图");

        // 等待画图启动
        thread::sleep(Duration::from_secs(3));

        let uia = WindowsUiaBackend::new().expect("无法创建 UIA 后端");
        let mut backend = NativeBackend::default();

        let windows = uia.list_windows().expect("无法获取窗口列表");
        let paint_window = windows.iter().find(|w| {
            w.title.to_lowercase().contains("paint") || w.title.contains("画图")
        });

        if let Some(window) = paint_window {
            println!("✓ 找到画图窗口: {}", window.title);
            let tree = uia.get_tree(window.hwnd).expect("无法获取 UI 树");

            // 查找画布区域
            fn find_canvas(elem: &claude_codex_pro_core::claude_desktop_computer_use::uia::types::UiElement)
                -> Option<&claude_codex_pro_core::claude_desktop_computer_use::uia::types::UiElement> {
                use claude_codex_pro_core::claude_desktop_computer_use::uia::types::ElementType;

                if matches!(elem.element_type, ElementType::Group) &&
                   elem.label.contains("画布") {
                    return Some(elem);
                }

                for child in &elem.children {
                    if let Some(found) = find_canvas(child) {
                        return Some(found);
                    }
                }
                None
            }

            if let Some(canvas) = find_canvas(&tree) {
                println!("✓ 找到画布: {}", canvas.label);
                println!("  位置: x={}, y={}, w={}, h={}",
                    canvas.rect.x, canvas.rect.y,
                    canvas.rect.width, canvas.rect.height);

                // 绘制哆啦A梦 - 简化版，使用拖动分段绘制
                let center_x = canvas.rect.x + canvas.rect.width / 2;
                let center_y = canvas.rect.y + canvas.rect.height / 2 - 50;
                let radius = 80;

                println!("\n🎨 开始绘制简化版哆啦A梦...");

                // 1. 脸部圆形 - 分4段绘制
                println!("1. 绘制脸部圆形");
                draw_circle_segments(&mut backend, center_x, center_y, radius, 4);

                // 2. 左眼
                println!("2. 绘制左眼");
                draw_circle_segments(&mut backend, center_x - 25, center_y - 15, 12, 2);

                // 3. 右眼
                println!("3. 绘制右眼");
                draw_circle_segments(&mut backend, center_x + 25, center_y - 15, 12, 2);

                // 4. 鼻子
                println!("4. 绘制鼻子");
                draw_circle_segments(&mut backend, center_x, center_y, 8, 2);

                // 5. 嘴巴 - 简单的微笑弧线
                println!("5. 绘制嘴巴");
                let mouth_y = center_y + 20;
                backend.drag(
                    (center_x - 20, mouth_y),
                    (center_x, mouth_y + 10)
                ).expect("拖动失败");
                thread::sleep(Duration::from_millis(200));
                backend.drag(
                    (center_x, mouth_y + 10),
                    (center_x + 20, mouth_y)
                ).expect("拖动失败");
                thread::sleep(Duration::from_millis(200));

                // 6-11. 胡须
                println!("6. 绘制胡须");
                // 左边3根
                backend.drag((center_x - 30, center_y - 10), (center_x - 55, center_y - 20)).expect("拖动失败");
                thread::sleep(Duration::from_millis(150));
                backend.drag((center_x - 30, center_y), (center_x - 55, center_y)).expect("拖动失败");
                thread::sleep(Duration::from_millis(150));
                backend.drag((center_x - 30, center_y + 10), (center_x - 55, center_y + 20)).expect("拖动失败");
                thread::sleep(Duration::from_millis(150));

                // 右边3根
                backend.drag((center_x + 30, center_y - 10), (center_x + 55, center_y - 20)).expect("拖动失败");
                thread::sleep(Duration::from_millis(150));
                backend.drag((center_x + 30, center_y), (center_x + 55, center_y)).expect("拖动失败");
                thread::sleep(Duration::from_millis(150));
                backend.drag((center_x + 30, center_y + 10), (center_x + 55, center_y + 20)).expect("拖动失败");
                thread::sleep(Duration::from_millis(150));

                // 12. 身体圆形
                let body_y = center_y + radius + 80;
                println!("7. 绘制身体");
                draw_circle_segments(&mut backend, center_x, body_y, 90, 4);

                // 13. 铃铛
                println!("8. 绘制铃铛");
                draw_circle_segments(&mut backend, center_x, center_y + radius - 5, 10, 2);

                // 14-17. 口袋
                println!("9. 绘制口袋");
                let pocket_y = body_y + 20;
                // 口袋上边弧线
                backend.drag((center_x - 50, pocket_y), (center_x - 30, pocket_y - 10)).expect("拖动失败");
                thread::sleep(Duration::from_millis(200));
                backend.drag((center_x - 30, pocket_y - 10), (center_x, pocket_y - 15)).expect("拖动失败");
                thread::sleep(Duration::from_millis(200));
                backend.drag((center_x, pocket_y - 15), (center_x + 30, pocket_y - 10)).expect("拖动失败");
                thread::sleep(Duration::from_millis(200));
                backend.drag((center_x + 30, pocket_y - 10), (center_x + 50, pocket_y)).expect("拖动失败");
                thread::sleep(Duration::from_millis(200));

                // 口袋两边
                backend.drag((center_x - 50, pocket_y), (center_x - 50, pocket_y + 40)).expect("拖动失败");
                thread::sleep(Duration::from_millis(200));
                backend.drag((center_x + 50, pocket_y), (center_x + 50, pocket_y + 40)).expect("拖动失败");
                thread::sleep(Duration::from_millis(200));

                // 口袋底边
                backend.drag((center_x - 50, pocket_y + 40), (center_x + 50, pocket_y + 40)).expect("拖动失败");

                println!("\n✅ 哆啦A梦绘制完成！");

            } else {
                println!("❌ 未找到画布区域");
            }

        } else {
            println!("❌ 未找到画图窗口");
        }
    }

    // 一笔画圆：按住左键沿整圈采样点移动，最后只松开一次。
    // `segments` 仅用来按圆的大小决定采样密度（每段 15 个点）。
    fn draw_circle_segments(backend: &mut NativeBackend, center_x: i32, center_y: i32, radius: i32, segments: usize) {
        let total = segments.max(1) * 15;
        let points: Vec<(i32, i32)> = (0..=total)
            .map(|i| {
                let angle = (i as f64) * 2.0 * std::f64::consts::PI / (total as f64);
                (
                    center_x + (radius as f64 * angle.cos()).round() as i32,
                    center_y + (radius as f64 * angle.sin()).round() as i32,
                )
            })
            .collect();
        backend.drag_path(&points).expect("一笔画圆失败");
        thread::sleep(Duration::from_millis(200));
    }
}
