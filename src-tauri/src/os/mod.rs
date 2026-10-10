#[cfg(windows)]
pub mod windows;
#[cfg(windows)]
pub use windows as platform;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "linux")]
pub use linux as platform;

#[cfg(not(any(windows, target_os = "linux")))]
compile_error!("Sarab runs on Windows and Linux so far; macOS is PLAN.md Phase 3.");

#[derive(Default)]
pub struct Media {
    pub title: String,
    pub artist: String,
    pub album_title: String,
    pub album_artist: String,
    pub art: Vec<u8>,
    pub state: &'static str,
    pub position: f64,
    pub duration: f64,
}
