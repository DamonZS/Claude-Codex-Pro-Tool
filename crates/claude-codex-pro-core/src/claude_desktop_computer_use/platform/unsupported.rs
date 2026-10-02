use super::super::tools::{Backend, MouseButton, Screen, ScreenSize};

pub fn prepare_process() {}

#[derive(Default)]
pub struct NativeBackend;

fn unsupported<T>() -> anyhow::Result<T> {
    anyhow::bail!("当前平台不支持 Computer Use（仅支持 Windows 与 macOS）")
}

impl Backend for NativeBackend {
    fn screen_size(&self) -> anyhow::Result<ScreenSize> {
        unsupported()
    }
    fn capture(&self) -> anyhow::Result<Screen> {
        unsupported()
    }
    fn cursor_position(&self) -> anyhow::Result<(i32, i32)> {
        unsupported()
    }
    fn move_mouse(&mut self, _: i32, _: i32) -> anyhow::Result<()> {
        unsupported()
    }
    fn click(&mut self, _: i32, _: i32, _: MouseButton, _: u32) -> anyhow::Result<()> {
        unsupported()
    }
    fn drag(&mut self, _: (i32, i32), _: (i32, i32)) -> anyhow::Result<()> {
        unsupported()
    }
    fn scroll(&mut self, _: i32, _: i32, _: i32, _: i32) -> anyhow::Result<()> {
        unsupported()
    }
    fn type_text(&mut self, _: &str) -> anyhow::Result<()> {
        unsupported()
    }
    fn press_keys(&mut self, _: &[String]) -> anyhow::Result<()> {
        unsupported()
    }
}
