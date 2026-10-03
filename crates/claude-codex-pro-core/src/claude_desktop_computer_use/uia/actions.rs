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

    /// Set text using ValuePattern
    pub fn set_text(&self, element_id: &str, text: &str) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            let element = self.lookup(element_id)?;
            unsafe {
                let pattern: IUIAutomationValuePattern = element
                    .GetCurrentPatternAs(UIA_ValuePatternId)
                    .context("Element does not support Value pattern")?;

                let readonly = pattern
                    .CurrentIsReadOnly()
                    .context("Failed to check readonly status")?;
                if readonly.as_bool() {
                    return Err(anyhow!("Element is read-only"));
                }

                let bstr = BSTR::from(text);
                pattern.SetValue(&bstr).context("Failed to set text")?;
            }
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
        let child = std::process::Command::new("notepad.exe")
            .spawn()
            .expect("Failed to launch notepad");
        let hwnd_usize = child.id() as usize;
        std::thread::sleep(std::time::Duration::from_secs(2));

        let backend = WindowsUiaBackend::new().unwrap();
        let tree = backend.get_tree(hwnd_usize).unwrap();

        // Find File menu (usually first MenuItem)
        let menu_item = tree.children.iter()
            .find(|c| matches!(c.element_type, super::super::types::ElementType::MenuItem))
            .expect("No MenuItem found");

        // Click should succeed
        let result = backend.click_element(&menu_item.id);

        // Cleanup
        let _ = std::process::Command::new("taskkill")
            .args(&["/PID", &child.id().to_string(), "/F"])
            .output();

        assert!(result.is_ok(), "Click failed: {:?}", result.err());
    }

    #[test]
    #[ignore]
    fn test_set_text() {
        let child = std::process::Command::new("notepad.exe")
            .spawn()
            .expect("Failed to launch notepad");
        let hwnd_usize = child.id() as usize;
        std::thread::sleep(std::time::Duration::from_secs(2));

        let backend = WindowsUiaBackend::new().unwrap();
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
            .args(&["/PID", &child.id().to_string(), "/F"])
            .output();

        assert!(
            edit2.value.as_deref().unwrap_or("").contains(test_text),
            "Text not set correctly: {:?}",
            edit2.value
        );
    }
}
