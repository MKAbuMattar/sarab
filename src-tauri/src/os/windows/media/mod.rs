//! Sound and media: other apps playing, loopback capture, what is playing.

pub(in crate::os::windows) use super::*;

mod audio;
pub use audio::*;
mod now_playing;
pub use now_playing::*;
