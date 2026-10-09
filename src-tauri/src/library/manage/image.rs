use super::*;

pub const CUSTOM_THUMBNAIL: &str = "thumbnail-custom";
pub const FRAME_THUMBNAIL: &str = "thumbnail-frame.png";
pub const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;
pub const MAX_IMAGE_SIDE: u32 = 8000;

fn be16(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from(*b.get(i)?) << 8 | u32::from(*b.get(i + 1)?))
}

fn be32(b: &[u8], i: usize) -> Option<u32> {
    Some(be16(b, i)? << 16 | be16(b, i + 2)?)
}

fn le16(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from(*b.get(i)?) | u32::from(*b.get(i + 1)?) << 8)
}

fn le24(b: &[u8], i: usize) -> Option<u32> {
    Some(le16(b, i)? | u32::from(*b.get(i + 2)?) << 16)
}

fn png_size(b: &[u8]) -> Option<(u32, u32)> {
    (b.get(12..16)? == b"IHDR").then_some(())?;
    Some((be32(b, 16)?, be32(b, 20)?))
}

fn jpeg_size(b: &[u8]) -> Option<(u32, u32)> {
    let mut i = 2;
    loop {
        while *b.get(i)? == 0xFF && *b.get(i + 1)? == 0xFF {
            i += 1;
        }
        if *b.get(i)? != 0xFF {
            return None;
        }
        let marker = *b.get(i + 1)?;
        if (0xC0..=0xCF).contains(&marker) && !matches!(marker, 0xC4 | 0xC8 | 0xCC) {
            return Some((be16(b, i + 7)?, be16(b, i + 5)?));
        }
        if marker == 0xDA || marker == 0xD9 {
            return None;
        }
        i += 2 + be16(b, i + 2)? as usize;
    }
}

fn webp_size(b: &[u8]) -> Option<(u32, u32)> {
    match b.get(12..16)? {
        b"VP8 " => {
            (b.get(23..26)? == [0x9D, 0x01, 0x2A]).then_some(())?;
            Some((le16(b, 26)? & 0x3FFF, le16(b, 28)? & 0x3FFF))
        }
        b"VP8L" => {
            (*b.get(20)? == 0x2F).then_some(())?;
            let bits = u32::from(*b.get(21)?)
                | u32::from(*b.get(22)?) << 8
                | u32::from(*b.get(23)?) << 16
                | u32::from(*b.get(24)?) << 24;
            Some(((bits & 0x3FFF) + 1, ((bits >> 14) & 0x3FFF) + 1))
        }
        b"VP8X" => Some((le24(b, 24)? + 1, le24(b, 27)? + 1)),
        _ => None,
    }
}

pub fn check_image(b: &[u8]) -> Result<&'static str, String> {
    if b.len() > MAX_IMAGE_BYTES {
        return Err("the image is larger than 10 MB".into());
    }
    let (ext, size) = if b.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        ("png", png_size(b))
    } else if b.starts_with(&[0xFF, 0xD8, 0xFF]) {
        ("jpg", jpeg_size(b))
    } else if b.starts_with(b"RIFF") && b.get(8..12) == Some(b"WEBP") {
        ("webp", webp_size(b))
    } else {
        return Err("the thumbnail must be a PNG, JPEG or WebP image".into());
    };
    match size {
        Some((w, h)) if (1..=MAX_IMAGE_SIDE).contains(&w) && (1..=MAX_IMAGE_SIDE).contains(&h) => {
            Ok(ext)
        }
        Some((w, h)) => Err(format!(
            "the image is {w} by {h}; a thumbnail can be at most {MAX_IMAGE_SIDE} by {MAX_IMAGE_SIDE}"
        )),
        None => Err("the image file is damaged".into()),
    }
}

pub fn custom_thumbnail(dir: &Path) -> Option<PathBuf> {
    ["png", "jpg", "webp"]
        .iter()
        .map(|e| dir.join(format!("{CUSTOM_THUMBNAIL}.{e}")))
        .find(|p| p.is_file())
}

pub fn save_custom_thumbnail(dir: &Path, bytes: &[u8]) -> Result<PathBuf, String> {
    let ext = check_image(bytes)?;
    let path = dir.join(format!("{CUSTOM_THUMBNAIL}.{ext}"));
    let tmp = dir.join(format!("{CUSTOM_THUMBNAIL}.tmp"));
    fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    for old in ["png", "jpg", "webp"] {
        let _ = fs::remove_file(dir.join(format!("{CUSTOM_THUMBNAIL}.{old}")));
    }
    fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    Ok(path)
}

pub fn frame_thumbnail(dir: &Path) -> Option<PathBuf> {
    Some(dir.join(FRAME_THUMBNAIL)).filter(|p| p.is_file())
}

pub fn save_frame_thumbnail(dir: &Path, bytes: &[u8]) -> Result<PathBuf, String> {
    if check_image(bytes)? != "png" {
        return Err("the frame must be a PNG image".into());
    }
    let path = dir.join(FRAME_THUMBNAIL);
    let tmp = dir.join(format!("{FRAME_THUMBNAIL}.tmp"));
    fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    Ok(path)
}
