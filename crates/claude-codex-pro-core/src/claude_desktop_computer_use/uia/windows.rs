//! Window management operations

use anyhow::Result;

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HWND;
#[cfg(target_os = "windows")]
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowThreadProcessId, SetForegroundWindow, ShowWindow, SW_RESTORE,
};

/// Focus/activate a window
#[cfg(target_os = "windows")]
pub unsafe fn focus_window(hwnd: HWND) -> Result<()> {
    unsafe {
        // Restore if minimized
        let _ = ShowWindow(hwnd, SW_RESTORE);

        // Try direct SetForegroundWindow
        if SetForegroundWindow(hwnd).as_bool() {
            return Ok(());
        }

        // Fallback: try again after a small delay
        std::thread::sleep(std::time::Duration::from_millis(50));
        let _ = SetForegroundWindow(hwnd);

        Ok(())
    }
}

/// Get process ID from window handle
#[cfg(target_os = "windows")]
pub unsafe fn get_window_pid(hwnd: HWND) -> u32 {
    unsafe {
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        pid
    }
}

/// Get executable name from PID
#[cfg(target_os = "windows")]
pub unsafe fn get_exe_name_from_pid(pid: u32) -> String {
    unsafe {
        if let Ok(handle) = OpenProcess(PROCESS_QUERY_INFORMATION, false, pid) {
            // For now, return empty string - full implementation would use QueryFullProcessImageNameW
            let _ = handle;
            String::new()
        } else {
            String::new()
        }
    }
}

/// Check if window is foreground
#[cfg(target_os = "windows")]
pub unsafe fn is_foreground_window(hwnd: HWND) -> bool {
    unsafe { GetForegroundWindow() == hwnd }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "windows")]
    fn test_get_window_pid() {
        // Test with null HWND
        let hwnd = HWND(std::ptr::null_mut());
        unsafe {
            let pid = get_window_pid(hwnd);
            // Null HWND should return 0
            assert_eq!(pid, 0);
        }
    }
}
