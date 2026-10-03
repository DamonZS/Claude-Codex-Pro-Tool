#[cfg(test)]
mod tests {
    use super::super::backend::WindowsUiaBackend;
    use super::super::types::*;

    #[test]
    #[cfg(target_os = "windows")]
    fn test_backend_initialization() {
        let backend = WindowsUiaBackend::new();
        assert!(backend.is_ok(), "Failed to initialize UIA backend");
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_list_windows() {
        let backend = WindowsUiaBackend::new().expect("Failed to create backend");
        let windows = backend.list_windows().expect("Failed to list windows");

        // Should have at least some windows
        assert!(!windows.is_empty(), "No windows found");

        // Check first window has title
        if let Some(first) = windows.first() {
            assert!(!first.title.is_empty(), "Window title should not be empty");
            assert!(first.hwnd > 0, "Window HWND should be valid");
        }
    }

    #[test]
    fn test_element_type_parsing() {
        assert_eq!("Button".parse::<ElementType>().unwrap(), ElementType::Button);
        assert_eq!("button".parse::<ElementType>().unwrap(), ElementType::Button);
        assert_eq!("Edit".parse::<ElementType>().unwrap(), ElementType::Edit);
        assert_eq!("textbox".parse::<ElementType>().unwrap(), ElementType::Edit);
        assert_eq!("input".parse::<ElementType>().unwrap(), ElementType::Edit);
        assert_eq!("CheckBox".parse::<ElementType>().unwrap(), ElementType::CheckBox);
        assert_eq!("checkbutton".parse::<ElementType>().unwrap(), ElementType::CheckBox);
    }

    #[test]
    fn test_element_type_name() {
        assert_eq!(ElementType::Button.name(), "Button");
        assert_eq!(ElementType::Edit.name(), "Edit");
        assert_eq!(ElementType::CheckBox.name(), "CheckBox");
        assert_eq!(ElementType::Window.name(), "Window");
    }

    #[test]
    fn test_rect_helpers() {
        let rect = Rect {
            x: 10,
            y: 20,
            width: 100,
            height: 50,
        };

        assert_eq!(rect.left(), 10);
        assert_eq!(rect.top(), 20);
        assert_eq!(rect.right(), 110);
        assert_eq!(rect.bottom(), 70);
        assert!(!rect.is_empty());

        let empty_rect = Rect { x: 0, y: 0, width: 0, height: 0 };
        assert!(empty_rect.is_empty());
    }

    #[test]
    fn test_ui_element_creation() {
        let element = UiElement::new("test-id".to_string(), ElementType::Button);

        assert_eq!(element.id, "test-id");
        assert_eq!(element.element_type, ElementType::Button);
        assert!(element.enabled);
        assert!(!element.focused);
        assert!(element.children.is_empty());
    }
}
