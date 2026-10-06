//! PNG thumbnails through WIC and the shell.

use super::*;

pub(in crate::os::windows) fn wic() -> windows::core::Result<IWICImagingFactory> {
    use windows::Win32::Graphics::Imaging::CLSID_WICImagingFactory;
    use windows::Win32::System::Com::CLSCTX_INPROC_SERVER;
    unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER) }
}

/// Write `source` as a PNG no wider than `max` pixels, keeping its shape.
pub(in crate::os::windows) fn save_png(
    f: &IWICImagingFactory,
    source: &IWICBitmapSource,
    dest: &std::path::Path,
    max: u32,
) -> windows::core::Result<()> {
    use windows::core::Interface;
    use windows::Win32::Foundation::GENERIC_WRITE;
    use windows::Win32::Graphics::Imaging::{
        GUID_ContainerFormatPng, GUID_WICPixelFormat32bppBGRA, WICBitmapEncoderNoCache,
        WICBitmapInterpolationModeFant,
    };
    unsafe {
        let (mut w, mut h) = (0, 0);
        source.GetSize(&mut w, &mut h)?;
        let scaled: IWICBitmapSource = if w > max {
            let s = f.CreateBitmapScaler()?;
            s.Initialize(
                source,
                max,
                ((u64::from(h) * u64::from(max)) / u64::from(w.max(1))).max(1) as u32,
                WICBitmapInterpolationModeFant,
            )?;
            s.cast()?
        } else {
            source.clone()
        };
        scaled.GetSize(&mut w, &mut h)?;
        let stream = f.CreateStream()?;
        stream.InitializeFromFilename(&HSTRING::from(dest.as_os_str()), GENERIC_WRITE.0)?;
        let enc = f.CreateEncoder(&GUID_ContainerFormatPng, std::ptr::null())?;
        enc.Initialize(&stream, WICBitmapEncoderNoCache)?;
        let (mut frame, mut bag) = (None, None);
        enc.CreateNewFrame(&mut frame, &mut bag)?;
        let frame = frame.ok_or_else(windows::core::Error::empty)?;
        frame.Initialize(bag.as_ref())?;
        frame.SetSize(w, h)?;
        let mut fmt = GUID_WICPixelFormat32bppBGRA;
        frame.SetPixelFormat(&mut fmt)?;
        frame.WriteSource(&scaled, std::ptr::null())?;
        frame.Commit()?;
        enc.Commit()
    }
}

/// The Explorer thumbnail of `src` (a video, GIF or picture) saved as a PNG at `dest`.
pub fn shell_thumbnail(
    src: &std::path::Path,
    dest: &std::path::Path,
    size: i32,
) -> Result<(), String> {
    use windows::core::Interface;
    use windows::Win32::Foundation::SIZE;
    use windows::Win32::Graphics::Gdi::{DeleteObject, HPALETTE};
    use windows::Win32::Graphics::Imaging::WICBitmapIgnoreAlpha;
    use windows::Win32::UI::Shell::{
        IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_BIGGERSIZEOK,
    };
    let run = || -> windows::core::Result<()> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let item: IShellItemImageFactory =
                SHCreateItemFromParsingName(&HSTRING::from(src.as_os_str()), None)?;
            let hbm = item.GetImage(SIZE { cx: size, cy: size }, SIIGBF_BIGGERSIZEOK)?;
            let f = wic()?;
            // Thumbnails are opaque; some providers leave the alpha byte at 0.
            let bmp = f.CreateBitmapFromHBITMAP(hbm, HPALETTE::default(), WICBitmapIgnoreAlpha);
            let _ = DeleteObject(hbm.into());
            save_png(&f, &bmp?.cast()?, dest, size as u32)
        }
    };
    run().map_err(|e| format!("thumbnail of {}: {e}", src.display()))
}

/// Re-save the PNG at `src` as `dest`, no wider than `max` pixels.
pub fn shrink_png(src: &std::path::Path, dest: &std::path::Path, max: u32) -> Result<(), String> {
    use windows::core::Interface;
    use windows::Win32::Foundation::GENERIC_READ;
    use windows::Win32::Graphics::Imaging::WICDecodeMetadataCacheOnDemand;
    let run = || -> windows::core::Result<()> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let f = wic()?;
            let d = f.CreateDecoderFromFilename(
                &HSTRING::from(src.as_os_str()),
                None,
                GENERIC_READ,
                WICDecodeMetadataCacheOnDemand,
            )?;
            let frame = d.GetFrame(0)?;
            save_png(&f, &frame.cast()?, dest, max)
        }
    };
    run().map_err(|e| e.to_string())
}
