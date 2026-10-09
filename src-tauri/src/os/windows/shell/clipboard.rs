pub fn clipboard_text() -> Result<String, String> {
    use windows::Win32::Foundation::HGLOBAL;
    use windows::Win32::System::DataExchange::{
        CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
    };
    use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};
    const CF_UNICODETEXT: u32 = 13;
    unsafe {
        if IsClipboardFormatAvailable(CF_UNICODETEXT).is_err() {
            return Ok(String::new());
        }
        OpenClipboard(None).map_err(|e| e.to_string())?;
        let read = || -> windows::core::Result<String> {
            let h = HGLOBAL(GetClipboardData(CF_UNICODETEXT)?.0);
            let p = GlobalLock(h) as *const u16;
            if p.is_null() {
                return Ok(String::new());
            }
            let mut n = 0;
            while *p.add(n) != 0 {
                n += 1;
            }
            let text = String::from_utf16_lossy(std::slice::from_raw_parts(p, n));
            let _ = GlobalUnlock(h);
            Ok(text)
        };
        let r = read();
        let _ = CloseClipboard();
        r.map_err(|e| e.to_string())
    }
}
