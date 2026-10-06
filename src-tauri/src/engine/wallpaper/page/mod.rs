//! Talking to a playing page: calls, Customize values, volume and frame rate, feeds, video sync, mouse.

pub(in crate::engine::wallpaper) use super::*;

mod bridge;
pub use bridge::*;
mod props;
pub use props::*;
mod playback;
pub use playback::*;
mod feeds;
pub(in crate::engine::wallpaper) use feeds::*;
mod sync;
pub(in crate::engine::wallpaper) use sync::*;
mod mouse;
pub(in crate::engine::wallpaper) use mouse::*;
