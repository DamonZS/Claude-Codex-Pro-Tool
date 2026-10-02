#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(any(windows, target_os = "macos")))]
mod unsupported;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "macos")]
pub use macos::{NativeBackend, prepare_process};
#[cfg(not(any(windows, target_os = "macos")))]
pub use unsupported::{NativeBackend, prepare_process};
#[cfg(windows)]
pub use windows::{NativeBackend, prepare_process};

pub const SUPPORTED: bool = cfg!(any(windows, target_os = "macos"));
