//! What the Info view shows: size, files, dates.

use super::*;

/// Facts for the info dialog that the manifest does not hold.
#[derive(Serialize, Debug, Default)]
pub struct Details {
    /// Bytes in the package folder, plus the file it points to when that lives elsewhere.
    pub size: u64,
    pub files: u32,
    pub created: u64,
    /// When sarab.json was last written.
    pub modified: u64,
    pub folder: String,
    /// The file or web address it plays.
    pub source: String,
    pub has_props: bool,
}

pub fn details(w: &Wallpaper) -> Details {
    fn walk(dir: &Path, d: &mut Details) {
        for e in fs::read_dir(dir).into_iter().flatten().flatten() {
            // symlink_metadata: a link in a package is counted, never followed out of the folder.
            let Ok(m) = fs::symlink_metadata(e.path()) else {
                continue;
            };
            if m.is_dir() {
                walk(&e.path(), d);
            } else {
                d.size += m.len();
                d.files += 1;
            }
        }
    }
    let mut d = Details {
        created: w.added,
        modified: secs(
            fs::metadata(w.dir.join(INFO))
                .ok()
                .and_then(|m| m.modified().ok()),
        ),
        folder: dunce_str(&w.dir),
        has_props: w.has_props,
        ..Default::default()
    };
    walk(&w.dir, &mut d);
    match w.target() {
        Some(Target::Url(u)) => d.source = u,
        Some(Target::File(f)) => {
            if w.info.external {
                d.size += fs::metadata(&f).map_or(0, |m| m.len());
            }
            d.source = dunce_str(&f);
        }
        None => {}
    }
    d
}
