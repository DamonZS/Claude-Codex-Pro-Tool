use super::super::tools::{Backend, MouseButton, Screen, ScreenSize, is_modifier};
use std::time::Duration;
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CAPTUREBLT, CreateCompatibleBitmap,
    CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetDIBits, ReleaseDC,
    SRCCOPY, SelectObject,
};
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBD_EVENT_FLAGS, KEYBDINPUT,
    KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, MOUSE_EVENT_FLAGS,
    MOUSEEVENTF_HWHEEL, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN,
    MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL,
    MOUSEINPUT, SendInput, VIRTUAL_KEY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN, SetCursorPos,
};

const WHEEL_DELTA: i32 = 120;

/// Physical-pixel coordinates require per-monitor DPI awareness; otherwise
/// GetSystemMetrics/SetCursorPos are virtualized on scaled displays.
pub fn prepare_process() {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

#[derive(Default)]
pub struct NativeBackend;

fn send(inputs: &[INPUT]) -> anyhow::Result<()> {
    let sent = unsafe { SendInput(inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() {
        anyhow::bail!(
            "SendInput 只发送了 {sent}/{} 个事件（目标窗口可能以管理员权限运行）",
            inputs.len()
        );
    }
    Ok(())
}

fn mouse(flags: MOUSE_EVENT_FLAGS, data: i32) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: data as u32,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn key(vk: u16, scan: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk),
                wScan: scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn set_cursor(x: i32, y: i32) -> anyhow::Result<()> {
    unsafe { SetCursorPos(x, y)? };
    Ok(())
}

fn button_flags(button: MouseButton) -> (MOUSE_EVENT_FLAGS, MOUSE_EVENT_FLAGS) {
    match button {
        MouseButton::Left => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
        MouseButton::Right => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP),
        MouseButton::Middle => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP),
    }
}

/// Returns (virtual key, needs KEYEVENTF_EXTENDEDKEY).
fn virtual_key(name: &str) -> Option<(u16, bool)> {
    let code = match name {
        "ctrl" => 0x11,
        "shift" => 0x10,
        "alt" => 0x12,
        "meta" => return Some((0x5B, true)),
        "enter" => 0x0D,
        "tab" => 0x09,
        "escape" => 0x1B,
        "backspace" => 0x08,
        "space" => 0x20,
        "delete" => return Some((0x2E, true)),
        "insert" => return Some((0x2D, true)),
        "home" => return Some((0x24, true)),
        "end" => return Some((0x23, true)),
        "pageup" => return Some((0x21, true)),
        "pagedown" => return Some((0x22, true)),
        "left" => return Some((0x25, true)),
        "up" => return Some((0x26, true)),
        "right" => return Some((0x27, true)),
        "down" => return Some((0x28, true)),
        _ => {
            if let Some(n) = name.strip_prefix('f').and_then(|n| n.parse::<u16>().ok()) {
                if (1..=12).contains(&n) {
                    return Some((0x70 + n - 1, false));
                }
            }
            let mut chars = name.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) if c.is_ascii_alphanumeric() => c.to_ascii_uppercase() as u16,
                _ => return None,
            }
        }
    };
    Some((code, false))
}

impl Backend for NativeBackend {
    fn screen_size(&self) -> anyhow::Result<ScreenSize> {
        let (width, height) =
            unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) };
        if width <= 0 || height <= 0 {
            anyhow::bail!("无法获取主显示器尺寸");
        }
        Ok(ScreenSize {
            width: width as u32,
            height: height as u32,
        })
    }

    fn capture(&self) -> anyhow::Result<Screen> {
        let size = self.screen_size()?;
        let (w, h) = (size.width as i32, size.height as i32);
        let mut bgra = vec![0u8; (w * h * 4) as usize];
        unsafe {
            let screen_dc = GetDC(HWND::default());
            if screen_dc.is_invalid() {
                anyhow::bail!("GetDC 失败");
            }
            let mem_dc = CreateCompatibleDC(screen_dc);
            let bitmap = CreateCompatibleBitmap(screen_dc, w, h);
            let previous = SelectObject(mem_dc, bitmap);
            let blit = BitBlt(mem_dc, 0, 0, w, h, screen_dc, 0, 0, SRCCOPY | CAPTUREBLT);
            let mut info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w,
                    biHeight: -h, // top-down rows
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let lines = GetDIBits(
                mem_dc,
                bitmap,
                0,
                h as u32,
                Some(bgra.as_mut_ptr().cast()),
                &mut info,
                DIB_RGB_COLORS,
            );
            SelectObject(mem_dc, previous);
            let _ = DeleteObject(bitmap);
            let _ = DeleteDC(mem_dc);
            ReleaseDC(HWND::default(), screen_dc);
            blit?;
            if lines != h {
                anyhow::bail!("GetDIBits 失败");
            }
        }
        let rgb: Vec<u8> = bgra
            .chunks_exact(4)
            .flat_map(|px| [px[2], px[1], px[0]])
            .collect();
        let image = image::RgbImage::from_raw(size.width, size.height, rgb)
            .ok_or_else(|| anyhow::anyhow!("截图缓冲区尺寸不匹配"))?;
        Ok(Screen { size, image })
    }

    fn cursor_position(&self) -> anyhow::Result<(i32, i32)> {
        let mut point = POINT::default();
        unsafe { GetCursorPos(&mut point)? };
        Ok((point.x, point.y))
    }

    fn move_mouse(&mut self, x: i32, y: i32) -> anyhow::Result<()> {
        set_cursor(x, y)
    }

    fn click(&mut self, x: i32, y: i32, button: MouseButton, count: u32) -> anyhow::Result<()> {
        set_cursor(x, y)?;
        std::thread::sleep(Duration::from_millis(30));
        let (down, up) = button_flags(button);
        for _ in 0..count {
            send(&[mouse(down, 0), mouse(up, 0)])?;
            std::thread::sleep(Duration::from_millis(40));
        }
        Ok(())
    }

    fn drag(&mut self, from: (i32, i32), to: (i32, i32)) -> anyhow::Result<()> {
        set_cursor(from.0, from.1)?;
        std::thread::sleep(Duration::from_millis(30));
        send(&[mouse(MOUSEEVENTF_LEFTDOWN, 0)])?;
        let steps = 20;
        for step in 1..=steps {
            let x = from.0 + (to.0 - from.0) * step / steps;
            let y = from.1 + (to.1 - from.1) * step / steps;
            set_cursor(x, y)?;
            std::thread::sleep(Duration::from_millis(10));
        }
        send(&[mouse(MOUSEEVENTF_LEFTUP, 0)])
    }

    fn scroll(&mut self, x: i32, y: i32, dx: i32, dy: i32) -> anyhow::Result<()> {
        set_cursor(x, y)?;
        if dy != 0 {
            // Positive wheel data scrolls up on Windows; tool dy>0 means down.
            send(&[mouse(MOUSEEVENTF_WHEEL, -dy * WHEEL_DELTA)])?;
        }
        if dx != 0 {
            send(&[mouse(MOUSEEVENTF_HWHEEL, dx * WHEEL_DELTA)])?;
        }
        Ok(())
    }

    fn type_text(&mut self, text: &str) -> anyhow::Result<()> {
        for ch in text.chars() {
            if ch == '\n' {
                send(&[
                    key(0x0D, 0, KEYBD_EVENT_FLAGS(0)),
                    key(0x0D, 0, KEYEVENTF_KEYUP),
                ])?;
                continue;
            }
            if ch == '\r' {
                continue;
            }
            let mut units = [0u16; 2];
            let mut inputs = Vec::new();
            for unit in ch.encode_utf16(&mut units).iter() {
                inputs.push(key(0, *unit, KEYEVENTF_UNICODE));
                inputs.push(key(0, *unit, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP));
            }
            send(&inputs)?;
        }
        Ok(())
    }

    fn press_keys(&mut self, keys: &[String]) -> anyhow::Result<()> {
        let mut codes = Vec::new();
        for name in keys {
            let code =
                virtual_key(name).ok_or_else(|| anyhow::anyhow!("Windows 不支持按键 {name}"))?;
            codes.push((name.as_str(), code));
        }
        // Modifiers first, then the rest, released in reverse order.
        codes.sort_by_key(|(name, _)| !is_modifier(name));
        let flag = |extended: bool| {
            if extended {
                KEYEVENTF_EXTENDEDKEY
            } else {
                KEYBD_EVENT_FLAGS(0)
            }
        };
        let mut inputs: Vec<INPUT> = codes
            .iter()
            .map(|(_, (vk, ext))| key(*vk, 0, flag(*ext)))
            .collect();
        inputs.extend(
            codes
                .iter()
                .rev()
                .map(|(_, (vk, ext))| key(*vk, 0, flag(*ext) | KEYEVENTF_KEYUP)),
        );
        send(&inputs)
    }
}
