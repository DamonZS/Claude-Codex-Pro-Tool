//! macOS backend: CoreGraphics CGEvent for input, `screencapture` for pixels.
//! Requires Accessibility + Screen Recording for the responsible app
//! (Claude.app when launched from Claude Desktop).

use super::super::tools::{Backend, MouseButton, Screen, ScreenSize, is_modifier};
use std::ffi::c_void;
use std::time::Duration;

#[repr(C)]
#[derive(Clone, Copy)]
struct CGPoint {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CGSize {
    width: f64,
    height: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CGRect {
    origin: CGPoint,
    size: CGSize,
}

type CGEventRef = *mut c_void;

const TAP_HID: u32 = 0;
const SCROLL_UNIT_LINE: u32 = 1;
const FIELD_CLICK_STATE: u32 = 1;
const EV_LEFT_DOWN: u32 = 1;
const EV_LEFT_UP: u32 = 2;
const EV_RIGHT_DOWN: u32 = 3;
const EV_RIGHT_UP: u32 = 4;
const EV_MOUSE_MOVED: u32 = 5;
const EV_LEFT_DRAGGED: u32 = 6;
const EV_OTHER_DOWN: u32 = 25;
const EV_OTHER_UP: u32 = 26;
const FLAG_SHIFT: u64 = 0x0002_0000;
const FLAG_CTRL: u64 = 0x0004_0000;
const FLAG_ALT: u64 = 0x0008_0000;
const FLAG_CMD: u64 = 0x0010_0000;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGMainDisplayID() -> u32;
    fn CGDisplayBounds(display: u32) -> CGRect;
    fn CGEventCreate(source: *mut c_void) -> CGEventRef;
    fn CGEventGetLocation(event: CGEventRef) -> CGPoint;
    fn CGEventCreateMouseEvent(
        source: *mut c_void,
        kind: u32,
        point: CGPoint,
        button: u32,
    ) -> CGEventRef;
    fn CGEventCreateKeyboardEvent(source: *mut c_void, key: u16, down: bool) -> CGEventRef;
    fn CGEventCreateScrollWheelEvent2(
        source: *mut c_void,
        units: u32,
        wheel_count: u32,
        wheel1: i32,
        wheel2: i32,
        wheel3: i32,
    ) -> CGEventRef;
    fn CGEventSetIntegerValueField(event: CGEventRef, field: u32, value: i64);
    fn CGEventSetFlags(event: CGEventRef, flags: u64);
    fn CGEventKeyboardSetUnicodeString(event: CGEventRef, length: usize, string: *const u16);
    fn CGEventPost(tap: u32, event: CGEventRef);
    fn CGWarpMouseCursorPosition(point: CGPoint) -> i32;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(value: *const c_void);
}

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> bool;
}

pub fn prepare_process() {}

#[derive(Default)]
pub struct NativeBackend;

fn post(event: CGEventRef) -> anyhow::Result<()> {
    if event.is_null() {
        anyhow::bail!("创建 CGEvent 失败");
    }
    unsafe {
        CGEventPost(TAP_HID, event);
        CFRelease(event as *const c_void);
    }
    Ok(())
}

fn ensure_trusted() -> anyhow::Result<()> {
    if unsafe { AXIsProcessTrusted() } {
        Ok(())
    } else {
        anyhow::bail!(
            "macOS 未授予「辅助功能」权限：请在 系统设置 → 隐私与安全性 → 辅助功能 中允许 Claude，然后完全退出并重开 Claude Desktop。"
        )
    }
}

fn point(x: i32, y: i32) -> CGPoint {
    CGPoint {
        x: x as f64,
        y: y as f64,
    }
}

fn mouse_event(kind: u32, at: CGPoint, button: u32, clicks: i64) -> anyhow::Result<()> {
    let event = unsafe { CGEventCreateMouseEvent(std::ptr::null_mut(), kind, at, button) };
    if !event.is_null() && clicks > 0 {
        unsafe { CGEventSetIntegerValueField(event, FIELD_CLICK_STATE, clicks) };
    }
    post(event)
}

fn key_event(code: u16, down: bool, flags: u64) -> anyhow::Result<()> {
    let event = unsafe { CGEventCreateKeyboardEvent(std::ptr::null_mut(), code, down) };
    if !event.is_null() {
        unsafe { CGEventSetFlags(event, flags) };
    }
    post(event)
}

fn modifier_flag(name: &str) -> u64 {
    match name {
        "shift" => FLAG_SHIFT,
        "ctrl" => FLAG_CTRL,
        "alt" => FLAG_ALT,
        "meta" => FLAG_CMD,
        _ => 0,
    }
}

fn key_code(name: &str) -> Option<u16> {
    const LETTERS: [u16; 26] = [
        0, 11, 8, 2, 14, 3, 5, 4, 34, 38, 40, 37, 46, 45, 31, 35, 12, 15, 1, 17, 32, 9, 13, 7, 16,
        6,
    ];
    const DIGITS: [u16; 10] = [29, 18, 19, 20, 21, 23, 22, 26, 28, 25];
    const FKEYS: [u16; 12] = [122, 120, 99, 118, 96, 97, 98, 100, 101, 109, 103, 111];
    let code = match name {
        "meta" => 55,
        "shift" => 56,
        "alt" => 58,
        "ctrl" => 59,
        "enter" => 36,
        "tab" => 48,
        "space" => 49,
        "backspace" => 51,
        "escape" => 53,
        "delete" => 117,
        "home" => 115,
        "end" => 119,
        "pageup" => 116,
        "pagedown" => 121,
        "left" => 123,
        "right" => 124,
        "down" => 125,
        "up" => 126,
        _ => {
            if let Some(n) = name.strip_prefix('f').and_then(|n| n.parse::<usize>().ok()) {
                return FKEYS.get(n.checked_sub(1)?).copied();
            }
            let mut chars = name.chars();
            return match (chars.next(), chars.next()) {
                (Some(c @ 'a'..='z'), None) => Some(LETTERS[(c as u8 - b'a') as usize]),
                (Some(c @ '0'..='9'), None) => Some(DIGITS[(c as u8 - b'0') as usize]),
                _ => None,
            };
        }
    };
    Some(code)
}

impl Backend for NativeBackend {
    fn screen_size(&self) -> anyhow::Result<ScreenSize> {
        let bounds = unsafe { CGDisplayBounds(CGMainDisplayID()) };
        if bounds.size.width < 1.0 || bounds.size.height < 1.0 {
            anyhow::bail!("无法获取主显示器尺寸");
        }
        Ok(ScreenSize {
            width: bounds.size.width.round() as u32,
            height: bounds.size.height.round() as u32,
        })
    }

    fn capture(&self) -> anyhow::Result<Screen> {
        let size = self.screen_size()?;
        let path = std::env::temp_dir().join(format!("ccp-cu-{}.png", uuid::Uuid::new_v4()));
        let status = std::process::Command::new("/usr/sbin/screencapture")
            .args(["-x", "-m", "-t", "png"])
            .arg(&path)
            .status();
        let decoded = match status {
            Ok(status) if status.success() => image::open(&path).map(|img| img.to_rgb8()),
            Ok(status) => {
                let _ = std::fs::remove_file(&path);
                anyhow::bail!(
                    "screencapture 退出码 {status}：请在 系统设置 → 隐私与安全性 → 屏幕录制 中允许 Claude"
                );
            }
            Err(error) => anyhow::bail!("无法运行 screencapture：{error}"),
        };
        let _ = std::fs::remove_file(&path);
        let image = decoded
            .map_err(|error| anyhow::anyhow!("读取截图失败（可能缺少屏幕录制权限）：{error}"))?;
        Ok(Screen { size, image })
    }

    fn cursor_position(&self) -> anyhow::Result<(i32, i32)> {
        let event = unsafe { CGEventCreate(std::ptr::null_mut()) };
        if event.is_null() {
            anyhow::bail!("无法读取鼠标位置");
        }
        let location = unsafe { CGEventGetLocation(event) };
        unsafe { CFRelease(event as *const c_void) };
        Ok((location.x.round() as i32, location.y.round() as i32))
    }

    fn move_mouse(&mut self, x: i32, y: i32) -> anyhow::Result<()> {
        ensure_trusted()?;
        mouse_event(EV_MOUSE_MOVED, point(x, y), 0, 0)
    }

    fn click(&mut self, x: i32, y: i32, button: MouseButton, count: u32) -> anyhow::Result<()> {
        ensure_trusted()?;
        let at = point(x, y);
        let (down, up, index) = match button {
            MouseButton::Left => (EV_LEFT_DOWN, EV_LEFT_UP, 0),
            MouseButton::Right => (EV_RIGHT_DOWN, EV_RIGHT_UP, 1),
            MouseButton::Middle => (EV_OTHER_DOWN, EV_OTHER_UP, 2),
        };
        unsafe { CGWarpMouseCursorPosition(at) };
        mouse_event(EV_MOUSE_MOVED, at, 0, 0)?;
        std::thread::sleep(Duration::from_millis(30));
        for click in 1..=count as i64 {
            mouse_event(down, at, index, click)?;
            mouse_event(up, at, index, click)?;
            std::thread::sleep(Duration::from_millis(40));
        }
        Ok(())
    }

    fn drag(&mut self, from: (i32, i32), to: (i32, i32)) -> anyhow::Result<()> {
        ensure_trusted()?;
        let start = point(from.0, from.1);
        unsafe { CGWarpMouseCursorPosition(start) };
        mouse_event(EV_LEFT_DOWN, start, 0, 1)?;
        let steps = 20;
        for step in 1..=steps {
            let x = from.0 + (to.0 - from.0) * step / steps;
            let y = from.1 + (to.1 - from.1) * step / steps;
            mouse_event(EV_LEFT_DRAGGED, point(x, y), 0, 0)?;
            std::thread::sleep(Duration::from_millis(10));
        }
        mouse_event(EV_LEFT_UP, point(to.0, to.1), 0, 1)
    }

    fn scroll(&mut self, x: i32, y: i32, dx: i32, dy: i32) -> anyhow::Result<()> {
        ensure_trusted()?;
        let at = point(x, y);
        unsafe { CGWarpMouseCursorPosition(at) };
        mouse_event(EV_MOUSE_MOVED, at, 0, 0)?;
        // wheel1 is vertical (positive = up), wheel2 horizontal (positive = left).
        let event = unsafe {
            CGEventCreateScrollWheelEvent2(std::ptr::null_mut(), SCROLL_UNIT_LINE, 2, -dy, -dx, 0)
        };
        post(event)
    }

    fn type_text(&mut self, text: &str) -> anyhow::Result<()> {
        ensure_trusted()?;
        for line_part in text.split_inclusive('\n') {
            let (body, newline) = match line_part.strip_suffix('\n') {
                Some(body) => (body, true),
                None => (line_part, false),
            };
            let units: Vec<u16> = body.trim_end_matches('\r').encode_utf16().collect();
            // CGEventKeyboardSetUnicodeString accepts at most ~20 units per event.
            for chunk in units.chunks(16) {
                for down in [true, false] {
                    let event =
                        unsafe { CGEventCreateKeyboardEvent(std::ptr::null_mut(), 0, down) };
                    if !event.is_null() {
                        unsafe {
                            CGEventKeyboardSetUnicodeString(event, chunk.len(), chunk.as_ptr())
                        };
                    }
                    post(event)?;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            if newline {
                key_event(36, true, 0)?;
                key_event(36, false, 0)?;
            }
        }
        Ok(())
    }

    fn press_keys(&mut self, keys: &[String]) -> anyhow::Result<()> {
        ensure_trusted()?;
        let mut ordered: Vec<&String> = keys.iter().collect();
        ordered.sort_by_key(|name| !is_modifier(name));
        let mut codes = Vec::new();
        for name in &ordered {
            let code = key_code(name).ok_or_else(|| anyhow::anyhow!("macOS 不支持按键 {name}"))?;
            codes.push((name.as_str(), code));
        }
        let mut flags = 0u64;
        for (name, code) in &codes {
            flags |= modifier_flag(name);
            key_event(*code, true, flags)?;
        }
        for (name, code) in codes.iter().rev() {
            key_event(*code, false, flags)?;
            flags &= !modifier_flag(name);
        }
        Ok(())
    }
}
