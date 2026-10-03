//! Element action implementations (click, set_text, focus, etc.)

use anyhow::{anyhow, Context, Result};

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::POINT;
#[cfg(target_os = "windows")]
use windows::Win32::UI::Accessibility::{
    IUIAutomationElement, IUIAutomationInvokePattern, IUIAutomationValuePattern,
    UIA_InvokePatternId, UIA_ValuePatternId,
};
#[cfg(target_os = "windows")]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEINPUT,
};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, SetCursorPos};

/// Click an element at its center point
#[cfg(target_os = "windows")]
pub unsafe fn click_element(element: &IUIAutomationElement) -> Result<()> {
    unsafe {
        // Try InvokePattern first (preferred for buttons, menu items, etc.)
        if let Ok(pattern) = element
            .GetCurrentPatternAs::<IUIAutomationInvokePattern>(UIA_InvokePatternId)
        {
            return pattern
                .Invoke()
                .context("Failed to invoke element via InvokePattern");
        }

        // Fallback: physical mouse click at element center
        if let Ok(rect) = element.CurrentBoundingRectangle() {
            let center_x = rect.left + (rect.right - rect.left) / 2;
            let center_y = rect.top + (rect.bottom - rect.top) / 2;

            click_at(center_x, center_y)?;
            return Ok(());
        }

        Err(anyhow!("Element has no InvokePattern and no bounding box"))
    }
}

/// Set text in an element (Edit, ComboBox, etc.)
#[cfg(target_os = "windows")]
pub unsafe fn set_text(element: &IUIAutomationElement, text: &str) -> Result<()> {
    unsafe {
        let pattern = element
            .GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId)
            .context("Element does not support ValuePattern")?;

        let bstr = windows::core::BSTR::from(text);
        pattern
            .SetValue(&bstr)
            .context("Failed to set value via ValuePattern")
    }
}

/// Focus an element
#[cfg(target_os = "windows")]
pub unsafe fn focus_element(element: &IUIAutomationElement) -> Result<()> {
    unsafe {
        element
            .SetFocus()
            .context("Failed to set focus on element")
    }
}

/// Click at screen coordinates
#[cfg(target_os = "windows")]
fn click_at(x: i32, y: i32) -> Result<()> {
    unsafe {
        // Save current cursor position
        let mut old_pos = POINT::default();
        GetCursorPos(&mut old_pos).context("Failed to get cursor position")?;

        // Move to target
        SetCursorPos(x, y).context("Failed to move cursor")?;
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Mouse down
        let input_down = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_LEFTDOWN,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        // Mouse up
        let input_up = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_LEFTUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        let inputs = [input_down, input_up];
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);

        std::thread::sleep(std::time::Duration::from_millis(10));

        // Restore cursor
        SetCursorPos(old_pos.x, old_pos.y).ok();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "windows")]
    fn test_click_coordinates() {
        // Just ensure the function exists and compiles
        // Actual clicking would move the mouse
        let result = click_at(100, 100);
        assert!(result.is_ok());
    }
}
