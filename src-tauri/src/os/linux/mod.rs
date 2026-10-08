use crate::core::pause::Signals;
use std::path::{Path, PathBuf};

const NOT_YET: &str = "not available on Linux yet";

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RECT {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HWND(pub *mut std::ffi::c_void);

#[derive(Clone, Debug, PartialEq)]
pub struct Monitor {
    pub key: String,
    pub rect: RECT,
    pub work: RECT,
}

#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct Desktop {
    pub progman: isize,
    pub workerw: isize,
    pub defview: isize,
    pub raised: bool,
}

pub type Targets = std::sync::Arc<std::sync::Mutex<Vec<(RECT, isize)>>>;

pub struct AppProcess {
    pub pid: u32,
    pub hwnd: Option<isize>,
}

impl AppProcess {
    pub fn set_paused(&mut self, _paused: bool) -> Result<(), String> {
        Err(NOT_YET.into())
    }
}

pub fn monitors() -> Vec<Monitor> {
    Vec::new()
}

pub fn find_desktop() -> Option<Desktop> {
    None
}

pub fn desktop_alive(_d: &Desktop) -> bool {
    false
}

pub fn attach(_d: &Desktop, _hwnd: HWND, _r: RECT) -> Result<(), String> {
    Err(NOT_YET.into())
}

pub fn ensure_order(_d: &Desktop, _wins: &[HWND]) -> bool {
    false
}

pub fn show(_hwnd: HWND, _visible: bool) {}

pub fn refresh_desktop(_d: Option<&Desktop>) {}

pub fn handle(_w: &tauri::WebviewWindow) -> Option<HWND> {
    None
}

pub fn os_build() -> u32 {
    0
}

pub fn signals(mons: &[Monitor]) -> Signals {
    Signals {
        covered: vec![false; mons.len()],
        desktop_focused: true,
        ..Default::default()
    }
}

pub fn set_picture(_m: &Monitor, _path: &str) -> Result<(), String> {
    Err(NOT_YET.into())
}

pub fn get_picture(_m: &Monitor) -> Option<String> {
    None
}

pub fn cpu_times() -> (u64, u64) {
    let stat = std::fs::read_to_string("/proc/stat").unwrap_or_default();
    let v: Vec<u64> = stat
        .lines()
        .next()
        .unwrap_or_default()
        .split_whitespace()
        .skip(1)
        .filter_map(|n| n.parse().ok())
        .collect();
    let idle = v.get(3).copied().unwrap_or(0) + v.get(4).copied().unwrap_or(0);
    (idle, v.iter().sum::<u64>().saturating_sub(idle))
}

fn meminfo(key: &str) -> u64 {
    std::fs::read_to_string("/proc/meminfo")
        .unwrap_or_default()
        .lines()
        .find_map(|l| {
            l.strip_prefix(key)?
                .trim()
                .strip_suffix("kB")?
                .trim()
                .parse::<u64>()
                .ok()
        })
        .map_or(0, |kb| kb * 1024)
}

pub fn total_ram() -> u64 {
    meminfo("MemTotal:")
}

pub fn memory() -> (u64, u64) {
    let total = total_ram();
    (total, total.saturating_sub(meminfo("MemAvailable:")))
}

pub fn net_octets() -> (u64, u64) {
    (0, 0)
}

pub fn cpu_name() -> String {
    String::new()
}

pub fn gpu_name() -> String {
    String::new()
}

pub fn now_playing() -> Option<(String, String, String, String, Vec<u8>)> {
    None
}

pub fn last_input() -> (u32, u32) {
    (0, 0)
}

pub fn own_cpu_time() -> u64 {
    0
}

pub fn other_audio_playing() -> bool {
    false
}

pub fn is_sarab(pid: u32) -> bool {
    std::fs::read_link(format!("/proc/{pid}/exe"))
        .is_ok_and(|p| p.file_name().is_some_and(|n| n == "sarab"))
}

pub fn loopback(
    _wanted: &std::sync::atomic::AtomicBool,
    _each: &mut dyn FnMut(&[f32]),
) -> Result<(), String> {
    Err(NOT_YET.into())
}

pub fn pick_folder(_owner: Option<HWND>, _start: Option<&Path>) -> Option<PathBuf> {
    None
}

pub fn shell_thumbnail(_src: &Path, _dest: &Path, _size: i32) -> Result<(), String> {
    Err(NOT_YET.into())
}

pub fn shrink_png(_src: &Path, _dest: &Path, _max: u32) -> Result<(), String> {
    Err(NOT_YET.into())
}

pub fn capture_preview(
    _win: &tauri::WebviewWindow,
    _path: PathBuf,
    done: impl FnOnce(bool) + Send + 'static,
) {
    done(false);
}

pub fn update_toast_xml(_title: &str, _body: &str, _install: &str, _later: &str) -> String {
    String::new()
}

pub fn show_toast(
    _app_id: &str,
    _xml: &str,
    _on_answer: impl Fn(String) + Send + Sync + 'static,
) -> Result<(), String> {
    Err(NOT_YET.into())
}

pub fn set_on_path(_add: bool) -> Result<(), String> {
    Ok(())
}

pub fn tell_terminal(msg: &str) {
    eprintln!("{msg}");
}

pub fn recycle(_path: &Path) -> Result<(), String> {
    Err(NOT_YET.into())
}

pub fn launch_app(_exe: &Path, _args: &[&str]) -> Result<AppProcess, String> {
    Err("app wallpapers run on Windows only".into())
}

pub fn main_window(_pid: u32) -> Option<HWND> {
    None
}

pub fn input_window(_top: HWND) -> Option<isize> {
    None
}

pub fn window_rect(_h: HWND) -> Option<RECT> {
    None
}

pub fn forward_mouse(_targets: Targets, _running: std::sync::Arc<std::sync::atomic::AtomicBool>) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proc_readings() {
        let (idle, busy) = cpu_times();
        assert!(idle + busy > 0, "/proc/stat has CPU time");
        assert!(total_ram() > 0, "/proc/meminfo has MemTotal");
        let (total, used) = memory();
        assert!(used <= total);
        assert!(!is_sarab(std::process::id()), "a test binary is not sarab");
    }
}
