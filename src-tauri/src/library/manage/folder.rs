use super::*;

pub fn move_library(from: &Path, to: &Path) -> Result<usize, String> {
    let canon = |p: &Path| fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    fs::create_dir_all(to).map_err(|e| e.to_string())?;
    let (a, b) = (canon(from), canon(to));
    if a == b {
        return Ok(0);
    }
    if b.starts_with(&a) || a.starts_with(&b) {
        return Err("the new folder cannot be inside the library, or hold it".into());
    }
    let entries: Vec<PathBuf> = fs::read_dir(from)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| read(p).is_some())
        .collect();
    if let Some(taken) = entries
        .iter()
        .find(|p| to.join(p.file_name().unwrap_or_default()).exists())
    {
        return Err(format!(
            "{} already exists in the new folder",
            taken.file_name().unwrap_or_default().to_string_lossy()
        ));
    }
    for p in &entries {
        let dest = to.join(p.file_name().unwrap_or_default());
        if fs::rename(p, &dest).is_err() {
            copy_dir(p, &dest).map_err(|e| e.to_string())?;
            fs::remove_dir_all(p).map_err(|e| e.to_string())?;
        }
    }
    Ok(entries.len())
}
