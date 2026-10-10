use super::*;

pub fn asset_url(p: &Path) -> String {
    let s = p.to_string_lossy().replace('\\', "/");
    let mut out = format!("{ASSET_ORIGIN}/");
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' | b':' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

pub(in crate::engine::wallpaper) fn player_url(
    src: &str,
    kind: &str,
    fit: &str,
    clip: Option<[f64; 2]>,
) -> String {
    let q: String = src
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect();
    let range = clip.map_or(String::new(), |[a, b]| format!("&start={a}&end={b}"));
    format!("{APP_ORIGIN}/player.html?kind={kind}&fit={fit}{range}&src={q}")
}

pub(in crate::engine::wallpaper) fn youtube_ids(
    u: &str,
) -> Option<(Option<String>, Option<String>)> {
    let rest = u.split_once("://").map_or(u, |(_, r)| r);
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    let host = host
        .trim_start_matches("www.")
        .trim_start_matches("m.")
        .trim_start_matches("music.");
    let (path, query) = path.split_once('?').unwrap_or((path, ""));
    let id_chars = |s: &str| -> Option<String> {
        let id: String = s
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        (!id.is_empty()).then_some(id)
    };
    let param = |name: &str| {
        query
            .split('&')
            .find_map(|kv| kv.strip_prefix(name)?.strip_prefix('='))
            .and_then(id_chars)
    };
    let video = match host {
        "youtu.be" => id_chars(path),
        "youtube.com" | "youtube-nocookie.com" => match path.split_once('/') {
            Some(("shorts" | "live" | "embed" | "v", id)) => id_chars(id),
            _ if path == "watch" => param("v"),
            _ => None,
        },
        _ => return None,
    };
    let list = param("list");
    (video.is_some() || list.is_some()).then_some((video, list))
}

pub(in crate::engine::wallpaper) fn rewrite_url(u: &str) -> String {
    if let Some(id) = u.split("shadertoy.com/view/").nth(1) {
        return format!(
            "https://www.shadertoy.com/embed/{}?gui=false&paused=false&muted=true",
            id.trim_end_matches('/')
        );
    }
    if let Some((video, list)) = youtube_ids(u) {
        let mut q = vec![];
        q.extend(video.map(|v| format!("v={v}")));
        q.extend(list.map(|l| format!("list={l}")));
        return format!("{APP_ORIGIN}/youtube.html?{}", q.join("&"));
    }
    u.to_string()
}

pub(in crate::engine::wallpaper) fn url_for(
    app: &AppHandle,
    w: &Wallpaper,
    fit: &str,
) -> Result<tauri::Url, String> {
    let scope = app.asset_protocol_scope();
    let s = match (
        w.target().ok_or("wallpaper has no FileName")?,
        w.info.r#type,
    ) {
        (Target::Url(u), _) => rewrite_url(&u),
        (Target::File(f), k @ (Kind::Video | Kind::Gif | Kind::Picture)) => {
            if !f.is_file() {
                return Err(format!("missing file {}", f.display()));
            }
            scope.allow_file(&f).map_err(|e| e.to_string())?;
            player_url(
                &asset_url(&f),
                if k == Kind::Video { "video" } else { "gif" },
                fit,
                w.info.clip,
            )
        }
        (Target::File(f), Kind::Web) => {
            if !f.is_file() {
                return Err(format!("missing file {}", f.display()));
            }
            let root = if w.info.external {
                f.parent().unwrap_or(&f).to_path_buf()
            } else {
                w.dir.clone()
            };
            scope
                .allow_directory(&root, true)
                .map_err(|e| e.to_string())?;
            asset_url(&f)
        }
        (_, k) => return Err(format!("{} wallpapers are not supported yet", k.name())),
    };
    tauri::Url::parse(&s).map_err(|e| e.to_string())
}
