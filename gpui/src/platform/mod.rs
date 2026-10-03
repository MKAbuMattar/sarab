#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::*;
#[cfg(not(windows))]
compile_error!("Sarab has only the Windows backend so far; Linux and macOS come later.");
