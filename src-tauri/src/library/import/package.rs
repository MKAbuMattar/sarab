use super::*;

pub const MAX_UNPACKED: u64 = 2 << 30;

pub fn import_zip(lib: &Path, zip_path: &Path, max_bytes: u64) -> Result<Wallpaper, String> {
    let file = fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    let mut total = 0u64;
    for i in 0..zip.len() {
        let f = zip.by_index(i).map_err(|e| e.to_string())?;
        if f.enclosed_name().is_none() {
            return Err(format!("unsafe path in zip: {}", f.name()));
        }
        total += f.size();
    }
    if total > max_bytes {
        return Err(format!(
            "zip unpacks to {total} bytes, limit is {max_bytes}"
        ));
    }
    if zip.by_name(INFO).is_err() {
        return Err(format!("zip has no {INFO} at its root"));
    }
    let title = zip_path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let dir = new_dir(lib, &title).map_err(|e| e.to_string())?;
    let result = (|| -> Result<(), String> {
        for i in 0..zip.len() {
            let mut f = zip.by_index(i).map_err(|e| e.to_string())?;
            let rel = f.enclosed_name().ok_or("unsafe path")?;
            let out = dir.join(rel);
            if f.is_dir() {
                fs::create_dir_all(&out).map_err(|e| e.to_string())?;
                continue;
            }
            if let Some(p) = out.parent() {
                fs::create_dir_all(p).map_err(|e| e.to_string())?;
            }
            let mut w = fs::File::create(&out).map_err(|e| e.to_string())?;
            io::copy(&mut io::Read::take(&mut f, max_bytes), &mut w).map_err(|e| e.to_string())?;
        }
        Ok(())
    })();
    if let Err(e) = result {
        let _ = fs::remove_dir_all(&dir);
        return Err(e);
    }
    read(&dir).ok_or_else(|| {
        let _ = fs::remove_dir_all(&dir);
        format!("{INFO} in zip is invalid")
    })
}

pub fn export_zip(w: &Wallpaper, dest: &Path) -> Result<PathBuf, String> {
    use std::io::Write;
    let name: String = w
        .info
        .title
        .as_deref()
        .unwrap_or(&w.id)
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .take(60)
        .collect();
    let path = dest.join(format!("{}.zip", name.trim()));
    let mut info = w.info.clone();
    info.app_version
        .get_or_insert_with(|| env!("CARGO_PKG_VERSION").to_string());
    let outside = match w.target() {
        Some(Target::File(f)) if w.info.external => {
            let file = f
                .file_name()
                .ok_or("the file has no name")?
                .to_string_lossy()
                .into_owned();
            info.external = false;
            info.file = Some(file.clone());
            Some((f, file))
        }
        _ => None,
    };
    let result = (|| -> Result<(), String> {
        let mut zip = zip::ZipWriter::new(fs::File::create(&path).map_err(|e| e.to_string())?);
        let opts = zip::write::SimpleFileOptions::default();
        let err = |e: zip::result::ZipError| e.to_string();
        fn walk(dir: &Path, root: &Path, out: &mut Vec<PathBuf>) {
            for e in fs::read_dir(dir).into_iter().flatten().flatten() {
                let Ok(m) = fs::symlink_metadata(e.path()) else {
                    continue;
                };
                if m.is_dir() {
                    walk(&e.path(), root, out);
                } else if m.is_file() {
                    out.push(
                        e.path()
                            .strip_prefix(root)
                            .unwrap_or(&e.path())
                            .to_path_buf(),
                    );
                }
            }
        }
        let mut files = vec![];
        walk(&w.dir, &w.dir, &mut files);
        for rel in files.iter().filter(|r| r.as_os_str() != INFO) {
            zip.start_file(rel.to_string_lossy().replace('\\', "/"), opts)
                .map_err(err)?;
            io::copy(
                &mut fs::File::open(w.dir.join(rel)).map_err(|e| e.to_string())?,
                &mut zip,
            )
            .map_err(|e| e.to_string())?;
        }
        if let Some((src, file)) = &outside {
            zip.start_file(file.as_str(), opts).map_err(err)?;
            io::copy(
                &mut fs::File::open(src).map_err(|e| e.to_string())?,
                &mut zip,
            )
            .map_err(|e| e.to_string())?;
        }
        zip.start_file(INFO, opts).map_err(err)?;
        zip.write_all(&serde_json::to_vec_pretty(&info).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        zip.finish().map_err(err)?;
        Ok(())
    })();
    if let Err(e) = result {
        let _ = fs::remove_file(&path);
        return Err(e);
    }
    Ok(path)
}
