//! Changing what is in the library: edit, details, moving the folder, properties.json.

pub(in crate::library) use super::*;

mod edit;
pub use edit::*;
mod details;
pub use details::*;
mod folder;
pub use folder::*;
mod props;
pub use props::*;
