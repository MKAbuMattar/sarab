//! The desktop: wallpaper windows behind the icons, displays, the picture wallpaper, mouse input.

pub(in crate::os::windows) use super::*;

mod embed;
pub use embed::*;
mod displays;
pub use displays::*;
mod picture;
pub use picture::*;
mod mouse;
pub use mouse::*;
