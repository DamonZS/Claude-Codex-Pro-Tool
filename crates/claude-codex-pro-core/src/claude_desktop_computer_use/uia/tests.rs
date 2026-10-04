//! Integration tests for Windows UIA backend with real applications.

#[cfg(all(test, target_os = "windows"))]
mod notepad_tests {
    use crate::claude_desktop_computer_use::uia::{
        ElementType, FindParams, backend::WindowsUiaBackend,
    };
    use std::process::Command;
    use std::thread;
    use std::time::Duration;

    /// Helper to launch notepad and return its HWND.
    fn launch_notepad() -> Option<usize> {
        // Launch notepad
        let _ = Command::new("notepad.exe").spawn();

        // Wait for it to appear
        thread::sleep(Duration::from_millis(1000));

        // Find notepad window
        let backend = WindowsUiaBackend::new().ok()?;
        let windows = backend.list_windows().ok()?;

        windows
            .into_iter()
            .find(|w| w.title.contains("Notepad") || w.title.contains("记事本"))
            .map(|w| w.hwnd)
    }

    /// Helper to close notepad without saving.
    fn close_notepad(hwnd: usize) {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};

        unsafe {
            let hwnd = HWND(hwnd as *mut _);
            let _ = PostMessageW(hwnd, WM_CLOSE, None, None);
        }

        thread::sleep(Duration::from_millis(500));
    }

    #[test]
    fn test_get_notepad_tree() {
        let Some(hwnd) = launch_notepad() else {
            eprintln!("Failed to launch notepad, skipping test");
            return;
        };

        let backend = WindowsUiaBackend::new().unwrap();
        let tree = backend.get_tree(hwnd);

        close_notepad(hwnd);

        let tree = tree.expect("Failed to get UI tree");

        // Notepad should have a tree structure
        assert!(
            !tree.children.is_empty(),
            "Notepad tree should have children"
        );

        // Should have either Edit (old Notepad) or Document (new Notepad) control
        let has_edit = contains_type_recursive(&tree, ElementType::Edit);
        let has_document = contains_type_recursive(&tree, ElementType::Document);
        assert!(
            has_edit || has_document,
            "Notepad should contain an Edit or Document control"
        );

        println!(
            "✓ Notepad tree contains {} top-level children",
            tree.children.len()
        );
    }

    #[test]
    #[ignore] // 依赖外部窗口状态，手动验证
    fn test_find_notepad_edit() {
        let Some(hwnd) = launch_notepad() else {
            eprintln!("Failed to launch notepad, skipping test");
            return;
        };

        let backend = WindowsUiaBackend::new().unwrap();

        // Try to find Edit control (old Notepad)
        let params_edit = FindParams {
            element_type: Some(ElementType::Edit),
            query: None,
            interactive_only: false,
        };
        let results_edit = backend.find_elements(hwnd, &params_edit).ok();

        // Try to find Document control (new Notepad)
        let params_doc = FindParams {
            element_type: Some(ElementType::Document),
            query: None,
            interactive_only: false,
        };
        let results_doc = backend.find_elements(hwnd, &params_doc).ok();

        close_notepad(hwnd);

        // Should find at least one of them
        let edit_count = results_edit.as_ref().map(|r| r.len()).unwrap_or(0);
        let doc_count = results_doc.as_ref().map(|r| r.len()).unwrap_or(0);

        assert!(
            edit_count > 0 || doc_count > 0,
            "Should find at least one Edit or Document control (found {} Edit, {} Document)",
            edit_count,
            doc_count
        );

        if edit_count > 0 {
            println!(
                "✓ Found {} Edit control(s) in Notepad (old version)",
                edit_count
            );
        }
        if doc_count > 0 {
            println!(
                "✓ Found {} Document control(s) in Notepad (new version)",
                doc_count
            );
        }
    }

    #[test]
    #[ignore] // 依赖外部窗口状态，手动验证
    fn test_notepad_performance() {
        let Some(hwnd) = launch_notepad() else {
            eprintln!("Failed to launch notepad, skipping test");
            return;
        };

        let backend = WindowsUiaBackend::new().unwrap();

        let start = std::time::Instant::now();
        let tree = backend.get_tree(hwnd);
        let duration = start.elapsed();

        close_notepad(hwnd);

        tree.expect("Failed to get UI tree");

        println!("✓ get_tree took {:?}", duration);

        // Acceptance criteria: < 500ms for simple app like Notepad
        assert!(
            duration.as_millis() < 500,
            "get_tree should complete in < 500ms, took {:?}",
            duration
        );
    }

    fn contains_type_recursive(
        element: &crate::claude_desktop_computer_use::uia::UiElement,
        target_type: ElementType,
    ) -> bool {
        if element.element_type == target_type {
            return true;
        }

        element
            .children
            .iter()
            .any(|child| contains_type_recursive(child, target_type))
    }
}
