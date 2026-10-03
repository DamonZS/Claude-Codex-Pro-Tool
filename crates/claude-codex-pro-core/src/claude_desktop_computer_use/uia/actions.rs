//! Element interaction operations (click, set text, focus, etc.).

use anyhow::{anyhow, Result};

#[cfg(target_os = "windows")]
use windows::Win32::UI::Accessibility::{
    IUIAutomationElement, IUIAutomationInvokePattern, IUIAutomationValuePattern,
    UIA_InvokePatternId, UIA_ValuePatternId,
};

/// Click an element using the Invoke pattern.
#[cfg(target_os = "windows")]
pub unsafe fn click_element(element: &IUIAutomationElement) -> Result<()> {
    unsafe {
        // Try Invoke pattern first
        let pattern = element
            .GetCachedPatternAs::<IUIAutomationInvokePattern>(UIA_InvokePatternId)
            .ok();

        if let Some(invoke) = pattern {
            invoke
                .Invoke()
                .map_err(|e| anyhow!("Failed to invoke element: {}", e))?;
            return Ok(());
        }

        Err(anyhow!("Element does not support Invoke pattern"))
    }
}

/// Set text in an element using the Value pattern.
#[cfg(target_os = "windows")]
pub unsafe fn set_text(element: &IUIAutomationElement, text: &str) -> Result<()> {
    use windows::core::BSTR;

    unsafe {
        let pattern = element
            .GetCachedPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId)
            .map_err(|e| anyhow!("Element does not support Value pattern: {}", e))?;

        let bstr = BSTR::from(text);
        pattern
            .SetValue(&bstr)
            .map_err(|e| anyhow!("Failed to set value: {}", e))?;

        Ok(())
    }
}

/// Focus an element.
#[cfg(target_os = "windows")]
pub unsafe fn focus_element(element: &IUIAutomationElement) -> Result<()> {
    unsafe {
        element
            .SetFocus()
            .map_err(|e| anyhow!("Failed to set focus: {}", e))?;

        Ok(())
    }
}
