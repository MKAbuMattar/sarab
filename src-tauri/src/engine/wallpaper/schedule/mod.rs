//! What runs on the clock: the tick, cycling, the screensaver, unloading, CPU rest.

pub(in crate::engine::wallpaper) use super::*;

mod tick;
pub use tick::*;
mod cycle;
pub use cycle::*;
mod screensaver;
pub(in crate::engine::wallpaper) use screensaver::*;
mod unload;
pub(in crate::engine::wallpaper) use unload::*;
mod cpu;
pub(in crate::engine::wallpaper) use cpu::*;
