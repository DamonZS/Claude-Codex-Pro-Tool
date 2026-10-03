//! Element interaction actions for UI Automation.
//!
//! This module implements element operations using Windows UIA patterns:
//! - Click via InvokePattern
//! - Set text via ValuePattern
//! - Send keyboard input
//! - Toggle/expand/select operations

use anyhow::{anyhow, Context, Result};

#[cfg(target_os = "windows")]
use windows::core::BSTR;
#[cfg(target_os = "windows")]
use windows::Win32::UI::Accessibility::{
    IUIAutomationExpandCollapsePattern, IUIAutomationInvokePattern,
    IUIAutomationRangeValuePattern, IUIAutomationSelectionItemPattern,
    IUIAutomationTogglePattern, IUIAutomationValuePattern, UIA_ExpandCollapsePatternId,
    UIA_InvokePatternId, UIA_RangeValuePatternId, UIA_SelectionItemPatternId,
    UIA_TogglePatternId, UIA_ValuePatternId,
};

use super::backend::WindowsUiaBackend;
use super::keyboard::{clear_text_field, send_unicode_text, KeyboardTiming};

impl WindowsUiaBackend {
    /// Click an element using InvokePattern
    pub fn click_element(&self, element_id: &str) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            let element = self.lookup(element_id)?;
            unsafe {
                let pattern: IUIAutomationInvokePattern = element
                    .GetCurrentPatternAs(UIA_InvokePatternId)
                    .context("Element does not support Invoke pattern")?;
                pattern.Invoke().context("Failed to invoke element")?;
            }
            Ok(())
        }
        #[cfg(not(target_os = "windows"))]
        {
            Err(anyhow!("Windows UIA backend is only available on Windows"))
        }
    }

    /// Set text using ValuePattern, with keyboard fallback
    pub fn set_text(&self, element_id: &str, text: &str) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            let element = self.lookup(element_id)?;

            // Try ValuePattern first
            let pattern_result: windows::core::Result<IUIAutomationValuePattern> =
                unsafe { element.GetCurrentPatternAs(UIA_ValuePatternId) };

            if let Ok(pattern) = pattern_result {
                unsafe {
                    let readonly = pattern
                        .CurrentIsReadOnly()
                        .context("Failed to check readonly status")?;
                    if !readonly.as_bool() {
                        let bstr = BSTR::from(text);
                        if pattern.SetValue(&bstr).is_ok() {
                            return Ok(());
                        }
                    }
                }
            }

            // Fallback to keyboard input
            unsafe {
                element.SetFocus().context("Failed to focus element for keyboard input")?;
            }

            std::thread::sleep(std::time::Duration::from_millis(50));

            let timing = KeyboardTiming::default();
            clear_text_field(&timing).context("Failed to clear text field")?;
            send_unicode_text(text, &timing).context("Failed to send text via keyboard")?;

            Ok(())
        }
        #[cfg(not(target_os = "windows"))]
        {
            Err(anyhow!("Windows UIA backend is only available on Windows"))
        }
    }

    /// Send keyboard input (placeholder for Phase 2)
    pub fn send_keys(&self, _element_id: &str, _keys: &str) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            // Phase 2: Implement key parsing and SendInput
            Err(anyhow!("send_keys not yet implemented (Phase 2)"))
        }
        #[cfg(not(target_os = "windows"))]
        {
            Err(anyhow!("Windows UIA backend is only available on Windows"))
        }
    }

    /// Set focus to an element
    pub fn focus_element(&self, element_id: &str) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            let element = self.lookup(element_id)?;
            unsafe {
                element.SetFocus().context("Failed to focus element")?;
            }
            Ok(())
        }
        #[cfg(not(target_os = "windows"))]
        {
            Err(anyhow!("Windows UIA backend is only available on Windows"))
        }
    }

    /// Toggle an element (checkbox, toggle button)
    pub fn toggle_element(&self, element_id: &str) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            let element = self.lookup(element_id)?;
            unsafe {
                let pattern: IUIAutomationTogglePattern = element
                    .GetCurrentPatternAs(UIA_TogglePatternId)
                    .context("Element does not support Toggle pattern")?;
                pattern.Toggle().context("Failed to toggle element")?;
            }
            Ok(())
        }
        #[cfg(not(target_os = "windows"))]
        {
            Err(anyhow!("Windows UIA backend is only available on Windows"))
        }
    }

    /// Expand an element (tree item, combo box)
    pub fn expand_element(&self, element_id: &str) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            let element = self.lookup(element_id)?;
            unsafe {
                let pattern: IUIAutomationExpandCollapsePattern = element
                    .GetCurrentPatternAs(UIA_ExpandCollapsePatternId)
                    .context("Element does not support ExpandCollapse pattern")?;
                pattern.Expand().context("Failed to expand element")?;
            }
            Ok(())
        }
        #[cfg(not(target_os = "windows"))]
        {
            Err(anyhow!("Windows UIA backend is only available on Windows"))
        }
    }

    /// Collapse an element (tree item, combo box)
    pub fn collapse_element(&self, element_id: &str) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            let element = self.lookup(element_id)?;
            unsafe {
                let pattern: IUIAutomationExpandCollapsePattern = element
                    .GetCurrentPatternAs(UIA_ExpandCollapsePatternId)
                    .context("Element does not support ExpandCollapse pattern")?;
                pattern.Collapse().context("Failed to collapse element")?;
            }
            Ok(())
        }
        #[cfg(not(target_os = "windows"))]
        {
            Err(anyhow!("Windows UIA backend is only available on Windows"))
        }
    }

    /// Select an element (list item, tab item)
    pub fn select_element(&self, element_id: &str) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            let element = self.lookup(element_id)?;
            unsafe {
                let pattern: IUIAutomationSelectionItemPattern = element
                    .GetCurrentPatternAs(UIA_SelectionItemPatternId)
                    .context("Element does not support SelectionItem pattern")?;
                pattern.Select().context("Failed to select element")?;
            }
            Ok(())
        }
        #[cfg(not(target_os = "windows"))]
        {
            Err(anyhow!("Windows UIA backend is only available on Windows"))
        }
    }

    /// Set range value (slider, progress bar)
    pub fn set_range(&self, element_id: &str, value: f64) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            let element = self.lookup(element_id)?;
            unsafe {
                let pattern: IUIAutomationRangeValuePattern = element
                    .GetCurrentPatternAs(UIA_RangeValuePatternId)
                    .context("Element does not support RangeValue pattern")?;
                pattern.SetValue(value).context("Failed to set range value")?;
            }
            Ok(())
        }
        #[cfg(not(target_os = "windows"))]
        {
            Err(anyhow!("Windows UIA backend is only available on Windows"))
        }
    }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn test_click_element() {
        // Launch notepad
        let _child = std::process::Command::new("notepad.exe")
            .spawn()
            .expect("Failed to launch notepad");
        std::thread::sleep(std::time::Duration::from_millis(1500));

        let backend = WindowsUiaBackend::new().unwrap();

        // Find notepad window
        let windows = backend.list_windows().expect("Failed to list windows");
        let notepad_window = windows
            .into_iter()
            .find(|w| w.title.contains("Notepad") || w.title.contains("记事本"))
            .expect("Failed to find Notepad window");

        let hwnd_usize = notepad_window.hwnd;
        let tree = backend.get_tree(hwnd_usize).unwrap();

        // Find File menu (usually first MenuItem)
        let menu_item = tree.children.iter()
            .find(|c| matches!(c.element_type, super::super::types::ElementType::MenuItem))
            .expect("No MenuItem found");

        // Click should succeed
        let result = backend.click_element(&menu_item.id);

        // Cleanup
        let _ = std::process::Command::new("taskkill")
            .args(&["/IM", "notepad.exe", "/F"])
            .output();

        assert!(result.is_ok(), "Click failed: {:?}", result.err());
    }

    #[test]
    #[ignore]
    fn test_set_text() {
        let _child = std::process::Command::new("notepad.exe")
            .spawn()
            .expect("Failed to launch notepad");
        std::thread::sleep(std::time::Duration::from_millis(1500));

        let backend = WindowsUiaBackend::new().unwrap();

        // Find notepad window
        let windows = backend.list_windows().expect("Failed to list windows");
        let notepad_window = windows
            .into_iter()
            .find(|w| w.title.contains("Notepad") || w.title.contains("记事本"))
            .expect("Failed to find Notepad window");

        let hwnd_usize = notepad_window.hwnd;
        let tree = backend.get_tree(hwnd_usize).unwrap();

        let edit = tree.children.iter()
            .find(|c| matches!(c.element_type, super::super::types::ElementType::Edit))
            .expect("No Edit found");

        let test_text = "Hello UIA";
        backend.set_text(&edit.id, test_text).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(500));

        // Re-fetch tree to verify
        let tree2 = backend.get_tree(hwnd_usize).unwrap();
        let edit2 = tree2.children.iter()
            .find(|c| matches!(c.element_type, super::super::types::ElementType::Edit))
            .unwrap();

        // Cleanup
        let _ = std::process::Command::new("taskkill")
            .args(&["/IM", "notepad.exe", "/F"])
            .output();

        assert!(
            edit2.value.as_deref().unwrap_or("").contains(test_text),
            "Text not set correctly: {:?}",
            edit2.value
        );
    }
}
