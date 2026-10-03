pub mod actions;
pub mod backend;
pub mod element;
pub mod find;
pub mod keys;
pub mod registry;
pub mod screenshot;
pub mod types;
pub mod windows;

pub use backend::WindowsUiaBackend;
pub use types::*;

#[cfg(test)]
mod tests;
