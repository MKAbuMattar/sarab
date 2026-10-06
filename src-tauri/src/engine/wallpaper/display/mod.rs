//! Putting wallpapers on displays: windows, span, app wallpapers, where pages load from.

pub(in crate::engine::wallpaper) use super::*;

mod apply;
pub use apply::*;
mod span;
pub(in crate::engine::wallpaper) use span::*;
mod displays;
pub use displays::*;
mod app;
pub(in crate::engine::wallpaper) use app::*;
mod urls;
pub use urls::*;
mod labels;
pub(in crate::engine::wallpaper) use labels::*;
