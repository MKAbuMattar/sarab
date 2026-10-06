//! Commands for the settings window. The capability grants them to the "main" window only.

use super::*;

mod displays;
pub(crate) use displays::*;
mod wallpapers;
pub(crate) use wallpapers::*;
mod preferences;
pub(crate) use preferences::*;
mod system;
pub(crate) use system::*;
mod updates;
pub(crate) use updates::*;
