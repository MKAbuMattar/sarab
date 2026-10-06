//! What Sarab writes out: status.json, thumbnails, screenshots, the last frame.

pub(in crate::engine::wallpaper) use super::*;

mod status;
pub use status::*;
mod capture;
pub use capture::*;
