use super::*;

pub fn now_playing() -> Option<crate::os::Media> {
    use windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager as Manager;
    use windows::Media::Control::GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status;
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
    let status = session
        .GetPlaybackInfo()
        .and_then(|i| i.PlaybackStatus())
        .unwrap_or(Status::Closed);
    let state = match status {
        Status::Playing => "playing",
        Status::Paused => "paused",
        _ => "stopped",
    };
    let (position, duration) = session
        .GetTimelineProperties()
        .map(|t| {
            let ticks = |d: windows::core::Result<windows::Foundation::TimeSpan>| {
                d.map_or(0.0, |d| d.Duration as f64 / 1e7)
            };
            let start = ticks(t.StartTime());
            let mut at = ticks(t.Position()) - start;
            if state == "playing" {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0.0, |d| d.as_secs_f64() + 11_644_473_600.0);
                let updated = t
                    .LastUpdatedTime()
                    .map_or(now, |u| u.UniversalTime as f64 / 1e7);
                at += (now - updated).max(0.0);
            }
            let length = ticks(t.EndTime()) - start;
            (at.clamp(0.0, length.max(0.0)), length.max(0.0))
        })
        .unwrap_or((0.0, 0.0));
    Some(crate::os::Media {
        title: s(p.Title()),
        artist: s(p.Artist()),
        album_title: s(p.AlbumTitle()),
        album_artist: s(p.AlbumArtist()),
        art,
        state,
        position,
        duration,
    })
}
