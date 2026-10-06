//! What the system reports: pause signals, processes and CPU time, memory and hardware, app wallpapers.

pub(in crate::os::windows) use super::*;

mod probes;
pub use probes::*;
mod processes;
pub use processes::*;
mod sysinfo;
pub use sysinfo::*;
mod app_process;
pub use app_process::*;
