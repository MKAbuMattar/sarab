use crate::core::pause::Signals;
use std::path::{Path, PathBuf};

const NOT_YET: &str = "not available on Linux yet";

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RECT {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HWND(pub *mut std::ffi::c_void);

mod desktop;
pub use desktop::*;
mod media;
pub use media::*;
mod shell;
pub use shell::*;
mod system;
pub use system::*;

#[cfg(test)]
mod tests;
