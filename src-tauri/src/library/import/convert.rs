use super::*;

pub fn needs_convert(target: &str) -> bool {
    !target.contains("://")
        && Path::new(target)
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| {
                matches!(
                    e.to_ascii_lowercase().as_str(),
                    "avi" | "wmv" | "mpg" | "mpeg"
                )
            })
}

pub fn convert(lib: &Path, src: &Path) -> Result<PathBuf, String> {
    let abs = fs::canonicalize(src).map_err(|e| format!("{}: {e}", src.display()))?;
    let stem = src
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let dir = lib.join("converted").join(format!(
        "{:08x}",
        fnv(dunce_str(&abs).to_lowercase().as_bytes())
    ));
    let out = dir.join(format!("{stem}.mp4"));
    if out.is_file() {
        return Ok(out);
    }
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let part = dir.join("video.part");
    let mut cmd = std::process::Command::new("ffmpeg");
    cmd.args(["-y", "-v", "error", "-i"])
        .arg(&abs)
        .args([
            "-vf",
            "scale=trunc(iw/2)*2:trunc(ih/2)*2",
            "-c:v",
            "libx264",
            "-preset",
            "veryfast",
        ])
        .args([
            "-crf",
            "20",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-movflags",
            "+faststart",
            "-f",
            "mp4",
        ])
        .arg(&part);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let r = cmd.output().map_err(|e| match e.kind() {
        io::ErrorKind::NotFound => {
            "this video format needs ffmpeg: install it (winget install Gyan.FFmpeg) and try again"
                .to_string()
        }
        _ => format!("ffmpeg: {e}"),
    })?;
    if !r.status.success() {
        let _ = fs::remove_file(&part);
        return Err(format!(
            "ffmpeg could not convert {}: {}",
            src.display(),
            String::from_utf8_lossy(&r.stderr).trim()
        ));
    }
    fs::rename(&part, &out).map_err(|e| e.to_string())?;
    Ok(out)
}
