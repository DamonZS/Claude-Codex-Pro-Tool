use anyhow::{Context, Result};
use std::thread;
use std::time::Duration;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
    VIRTUAL_KEY, VK_CONTROL, VK_DELETE,
};

#[cfg(test)]
use windows::Win32::UI::Input::KeyboardAndMouse::VK_RETURN;

/// 键盘输入延迟配置
#[derive(Debug, Clone, Copy)]
pub struct KeyboardTiming {
    /// 按键之间的延迟（毫秒）
    pub key_delay_ms: u64,
    /// 按下和释放之间的延迟（毫秒）
    pub press_release_delay_ms: u64,
}

impl Default for KeyboardTiming {
    fn default() -> Self {
        Self {
            key_delay_ms: 10,
            press_release_delay_ms: 5,
        }
    }
}

/// 发送 Unicode 字符序列
pub fn send_unicode_text(text: &str, timing: &KeyboardTiming) -> Result<()> {
    for ch in text.chars() {
        send_unicode_char(ch, timing)?;
        if timing.key_delay_ms > 0 {
            thread::sleep(Duration::from_millis(timing.key_delay_ms));
        }
    }
    Ok(())
}

/// 发送单个 Unicode 字符
fn send_unicode_char(ch: char, timing: &KeyboardTiming) -> Result<()> {
    let mut inputs = Vec::new();

    // 按下
    inputs.push(create_unicode_input(ch, false));

    // 释放
    inputs.push(create_unicode_input(ch, true));

    unsafe {
        let result = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        if result as usize != inputs.len() {
            anyhow::bail!("SendInput failed for character: {}", ch);
        }
    }

    if timing.press_release_delay_ms > 0 {
        thread::sleep(Duration::from_millis(timing.press_release_delay_ms));
    }

    Ok(())
}

/// 创建 Unicode 输入结构
fn create_unicode_input(ch: char, key_up: bool) -> INPUT {
    let mut ki = KEYBDINPUT::default();
    ki.wScan = ch as u16;
    ki.dwFlags = KEYEVENTF_UNICODE;
    if key_up {
        ki.dwFlags |= KEYEVENTF_KEYUP;
    }

    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 { ki },
    }
}

/// 发送虚拟键码
pub fn send_virtual_key(vk: VIRTUAL_KEY, timing: &KeyboardTiming) -> Result<()> {
    let mut inputs = Vec::new();

    // 按下
    inputs.push(create_virtual_key_input(vk, false));

    // 释放
    inputs.push(create_virtual_key_input(vk, true));

    unsafe {
        let result = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        if result as usize != inputs.len() {
            anyhow::bail!("SendInput failed for virtual key: {:?}", vk);
        }
    }

    if timing.press_release_delay_ms > 0 {
        thread::sleep(Duration::from_millis(timing.press_release_delay_ms));
    }

    Ok(())
}

/// 创建虚拟键输入结构
fn create_virtual_key_input(vk: VIRTUAL_KEY, key_up: bool) -> INPUT {
    let mut ki = KEYBDINPUT::default();
    ki.wVk = vk;
    if key_up {
        ki.dwFlags = KEYEVENTF_KEYUP;
    }

    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 { ki },
    }
}

/// 发送快捷键组合（如 Ctrl+A）
pub fn send_key_combination(
    modifiers: &[VIRTUAL_KEY],
    key: VIRTUAL_KEY,
    timing: &KeyboardTiming,
) -> Result<()> {
    let mut inputs = Vec::new();

    // 按下所有修饰键
    for &modifier in modifiers {
        inputs.push(create_virtual_key_input(modifier, false));
    }

    // 按下主键
    inputs.push(create_virtual_key_input(key, false));

    // 释放主键
    inputs.push(create_virtual_key_input(key, true));

    // 释放所有修饰键（逆序）
    for &modifier in modifiers.iter().rev() {
        inputs.push(create_virtual_key_input(modifier, true));
    }

    unsafe {
        let result = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        if result as usize != inputs.len() {
            anyhow::bail!("SendInput failed for key combination");
        }
    }

    if timing.press_release_delay_ms > 0 {
        thread::sleep(Duration::from_millis(timing.press_release_delay_ms));
    }

    Ok(())
}

/// 清除文本框内容（Ctrl+A + Delete）
pub fn clear_text_field(timing: &KeyboardTiming) -> Result<()> {
    // Ctrl+A 全选
    send_key_combination(&[VK_CONTROL], VIRTUAL_KEY(b'A' as u16), timing)
        .context("Failed to send Ctrl+A")?;

    thread::sleep(Duration::from_millis(50));

    // Delete 删除
    send_virtual_key(VK_DELETE, timing).context("Failed to send Delete")?;

    thread::sleep(Duration::from_millis(50));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // 需要活动窗口，手动测试
    fn test_send_unicode_text() {
        let timing = KeyboardTiming::default();
        send_unicode_text("Hello 世界", &timing).unwrap();
    }

    #[test]
    #[ignore]
    fn test_send_virtual_key() {
        let timing = KeyboardTiming::default();
        send_virtual_key(VK_RETURN, &timing).unwrap();
    }

    #[test]
    #[ignore]
    fn test_clear_text_field() {
        let timing = KeyboardTiming::default();
        clear_text_field(&timing).unwrap();
    }
}
