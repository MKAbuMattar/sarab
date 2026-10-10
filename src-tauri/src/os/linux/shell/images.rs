use super::*;
use gdk::gdk_pixbuf::{InterpType, Pixbuf};

pub fn shell_thumbnail(src: &Path, dest: &Path, size: i32) -> Result<(), String> {
    let img = Pixbuf::from_file(src).map_err(|e| format!("{}: {e}", src.display()))?;
    save_scaled(&img, dest, size.max(1) as u32)
}

fn save_scaled(img: &Pixbuf, dest: &Path, max: u32) -> Result<(), String> {
    let (w, h) = (img.width().max(1), img.height().max(1));
    let k = (max as f64 / w.max(h) as f64).min(1.0);
    let (sw, sh) = (
        ((w as f64 * k).round() as i32).max(1),
        ((h as f64 * k).round() as i32).max(1),
    );
    let out = if k < 1.0 {
        img.scale_simple(sw, sh, InterpType::Bilinear)
            .ok_or("could not scale the image")?
    } else {
        img.clone()
    };
    out.savev(dest, "png", &[]).map_err(|e| e.to_string())
}

pub fn shrink_png(src: &Path, dest: &Path, max: u32) -> Result<(), String> {
    let img = Pixbuf::from_file(src).map_err(|e| e.to_string())?;
    save_scaled(&img, dest, max)
}

pub fn crop_png(src: &Path, dest: &Path, rect: [u32; 4], max: u32) -> Result<(), String> {
    let img = Pixbuf::from_file(src).map_err(|e| e.to_string())?;
    let [x, y, w, h] = rect.map(|v| v as i32);
    let (iw, ih) = (img.width(), img.height());
    let (x, y) = (x.clamp(0, iw - 1), y.clamp(0, ih - 1));
    let (w, h) = (w.clamp(1, iw - x), h.clamp(1, ih - y));
    save_scaled(&img.new_subpixbuf(x, y, w, h), dest, max)
}
