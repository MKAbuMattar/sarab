use super::*;

pub fn now_playing() -> Option<(String, String, String, String, Vec<u8>)> {
    use windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager as Manager;
    use windows::Storage::Streams::DataReader;
    let manager = Manager::RequestAsync().ok()?.join().ok()?;
    let session = manager.GetCurrentSession().ok()?;
    let p = session.TryGetMediaPropertiesAsync().ok()?.join().ok()?;
    let s = |r: windows::core::Result<HSTRING>| r.map(|h| h.to_string()).unwrap_or_default();
    let art = (|| -> windows::core::Result<Vec<u8>> {
        let stream = p.Thumbnail()?.OpenReadAsync()?.join()?;
        let size = stream.Size()?.min(4 << 20) as u32;
        let reader = DataReader::CreateDataReader(&stream.GetInputStreamAt(0)?)?;
        reader.LoadAsync(size)?.join()?;
        let mut bytes = vec![0u8; size as usize];
        reader.ReadBytes(&mut bytes)?;
        Ok(bytes)
    })()
    .unwrap_or_default();
    Some((
        s(p.Title()),
        s(p.Artist()),
        s(p.AlbumTitle()),
        s(p.AlbumArtist()),
        art,
    ))
}
