//! Integration test for UIA keyboard input functionality
//!
//! This test verifies that the keyboard fallback works correctly
//! when ValuePattern is not available or fails.

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use claude_codex_pro_core::claude_desktop_computer_use::uia::{
        ElementType, WindowsUiaBackend,
    };
    use std::process::{Child, Command};
    use std::thread;
    use std::time::Duration;

    struct NotepadGuard {
        child: Child,
    }

    impl NotepadGuard {
        fn new() -> Self {
            let child = Command::new("notepad.exe")
                .spawn()
                .expect("Failed to launch notepad");
            thread::sleep(Duration::from_secs(2));
            Self { child }
        }

        fn pid(&self) -> u32 {
            self.child.id()
        }
    }

    impl Drop for NotepadGuard {
        fn drop(&mut self) {
            let _ = Command::new("taskkill")
                .args(&["/PID", &self.pid().to_string(), "/F"])
                .output();
            thread::sleep(Duration::from_millis(500));
        }
    }

    #[test]
    #[ignore] // 需要 GUI 环境
    fn test_keyboard_input_integration() {
        let notepad = NotepadGuard::new();
        let hwnd = notepad.pid() as usize;

        let backend = WindowsUiaBackend::new().expect("Failed to create backend");
        let tree = backend.get_tree(hwnd).expect("Failed to get tree");

        let edit = tree
            .children
            .iter()
            .find(|c| matches!(c.element_type, ElementType::Edit))
            .expect("No Edit element found");

        // Test 1: ASCII text
        let ascii_text = "Hello World!";
        backend
            .set_text(&edit.id, ascii_text)
            .expect("Failed to set ASCII text");

        thread::sleep(Duration::from_millis(500));

        let tree2 = backend.get_tree(hwnd).expect("Failed to get tree");
        let edit2 = tree2
            .children
            .iter()
            .find(|c| matches!(c.element_type, ElementType::Edit))
            .unwrap();

        assert!(
            edit2.value.as_deref().unwrap_or("").contains(ascii_text),
            "ASCII text not set correctly"
        );

        // Test 2: Unicode text (中文)
        let unicode_text = "你好世界";
        backend
            .set_text(&edit.id, unicode_text)
            .expect("Failed to set Unicode text");

        thread::sleep(Duration::from_millis(500));

        let tree3 = backend.get_tree(hwnd).expect("Failed to get tree");
        let edit3 = tree3
            .children
            .iter()
            .find(|c| matches!(c.element_type, ElementType::Edit))
            .unwrap();

        assert!(
            edit3.value.as_deref().unwrap_or("").contains(unicode_text),
            "Unicode text not set correctly"
        );

        // Test 3: Mixed text
        let mixed_text = "Hello 世界 123!";
        backend
            .set_text(&edit.id, mixed_text)
            .expect("Failed to set mixed text");

        thread::sleep(Duration::from_millis(500));

        let tree4 = backend.get_tree(hwnd).expect("Failed to get tree");
        let edit4 = tree4
            .children
            .iter()
            .find(|c| matches!(c.element_type, ElementType::Edit))
            .unwrap();

        assert!(
            edit4.value.as_deref().unwrap_or("").contains(mixed_text),
            "Mixed text not set correctly"
        );
    }

    #[test]
    #[ignore]
    fn test_keyboard_clear_and_replace() {
        let notepad = NotepadGuard::new();
        let hwnd = notepad.pid() as usize;

        let backend = WindowsUiaBackend::new().expect("Failed to create backend");
        let tree = backend.get_tree(hwnd).expect("Failed to get tree");

        let edit = tree
            .children
            .iter()
            .find(|c| matches!(c.element_type, ElementType::Edit))
            .expect("No Edit element found");

        // Set initial text
        backend
            .set_text(&edit.id, "Initial text")
            .expect("Failed to set initial text");

        thread::sleep(Duration::from_millis(500));

        // Replace with new text
        backend
            .set_text(&edit.id, "Replaced text")
            .expect("Failed to replace text");

        thread::sleep(Duration::from_millis(500));

        let tree2 = backend.get_tree(hwnd).expect("Failed to get tree");
        let edit2 = tree2
            .children
            .iter()
            .find(|c| matches!(c.element_type, ElementType::Edit))
            .unwrap();

        let final_text = edit2.value.as_deref().unwrap_or("");
        assert!(
            final_text.contains("Replaced text"),
            "Text not replaced correctly: {}",
            final_text
        );
        assert!(
            !final_text.contains("Initial"),
            "Old text not cleared: {}",
            final_text
        );
    }
}
