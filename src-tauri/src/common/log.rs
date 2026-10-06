use super::cfg;
use crate::core::settings;
use std::fs;

pub fn log(msg: impl AsRef<str>) {
    use std::io::Write;
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let _ = fs::create_dir_all(settings::config_dir());
    if let Ok(mut f) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(cfg("sarab.log"))
    {
        let _ = writeln!(f, "{secs} {}", msg.as_ref());
    }
}
