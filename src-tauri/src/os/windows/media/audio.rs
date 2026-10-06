//! Sound: whether other apps play, and loopback capture for audio-reactive wallpapers.

use super::*;

/// Is another app making sound right now? Only sessions that are active and above a whisper
/// count, so a paused player or a silent stream does not mute the wallpaper.
pub fn other_audio_playing() -> bool {
    use windows::core::Interface;
    use windows::Win32::Media::Audio::Endpoints::IAudioMeterInformation;
    use windows::Win32::Media::Audio::{
        eConsole, eRender, AudioSessionStateActive, IAudioSessionControl2, IAudioSessionManager2,
        IMMDeviceEnumerator, MMDeviceEnumerator,
    };
    let own = own_processes();
    let run = || -> windows::core::Result<bool> {
        unsafe {
            let en: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let dev = en.GetDefaultAudioEndpoint(eRender, eConsole)?;
            let mgr: IAudioSessionManager2 = dev.Activate(CLSCTX_ALL, None)?;
            let list = mgr.GetSessionEnumerator()?;
            for i in 0..list.GetCount()? {
                let s = list.GetSession(i)?;
                if s.GetState()? != AudioSessionStateActive {
                    continue;
                }
                let pid = s.cast::<IAudioSessionControl2>()?.GetProcessId()?;
                // pid 0 is the system sounds session.
                if pid == 0 || own.contains(&pid) {
                    continue;
                }
                if s.cast::<IAudioMeterInformation>()?.GetPeakValue()? > 0.01 {
                    return Ok(true);
                }
            }
            Ok(false)
        }
    };
    run().unwrap_or(false)
}

/// Capture what the default output plays (WASAPI loopback) while `wanted`, as mono float
/// samples, and hand the last 1024 to `each` about every 33 ms. Silence is sent as zeros once.
pub fn loopback(
    wanted: &std::sync::atomic::AtomicBool,
    each: &mut dyn FnMut(&[f32]),
) -> windows::core::Result<()> {
    use std::sync::atomic::Ordering;
    use windows::Win32::Media::Audio::{
        eConsole, eRender, IAudioCaptureClient, IAudioClient, IMMDeviceEnumerator,
        MMDeviceEnumerator, AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED,
        AUDCLNT_STREAMFLAGS_LOOPBACK,
    };
    use windows::Win32::System::Com::CoTaskMemFree;
    unsafe {
        let _ = CoInitializeEx(None, windows::Win32::System::Com::COINIT_MULTITHREADED);
        let en: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let dev = en.GetDefaultAudioEndpoint(eRender, eConsole)?;
        let client: IAudioClient = dev.Activate(CLSCTX_ALL, None)?;
        let fmt = client.GetMixFormat()?;
        let (channels, bits, tag) = (
            (*fmt).nChannels as usize,
            (*fmt).wBitsPerSample,
            (*fmt).wFormatTag,
        );
        // The shared-mode mix format is 32-bit float on every Windows version Sarab supports.
        let float = bits == 32 && (tag == 3 || tag == 0xFFFE);
        let init = client.Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            AUDCLNT_STREAMFLAGS_LOOPBACK,
            10_000_000,
            0,
            fmt,
            None,
        );
        CoTaskMemFree(Some(fmt as *const _));
        init?;
        if !float || channels == 0 {
            return Err(windows::core::Error::new(
                windows::core::HRESULT(-1),
                "the mix format is not 32-bit float",
            ));
        }
        let cap: IAudioCaptureClient = client.GetService()?;
        client.Start()?;
        let mut ring: Vec<f32> = Vec::with_capacity(4096);
        let mut silent_sent = false;
        while wanted.load(Ordering::Relaxed) {
            std::thread::sleep(std::time::Duration::from_millis(33));
            let mut got = false;
            while cap.GetNextPacketSize()? > 0 {
                let (mut data, mut frames, mut flags) = (std::ptr::null_mut(), 0u32, 0u32);
                cap.GetBuffer(&mut data, &mut frames, &mut flags, None, None)?;
                if flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 == 0 && !data.is_null() {
                    let s =
                        std::slice::from_raw_parts(data as *const f32, frames as usize * channels);
                    ring.extend(
                        s.chunks(channels)
                            .map(|f| f.iter().sum::<f32>() / channels as f32),
                    );
                    got = true;
                }
                cap.ReleaseBuffer(frames)?;
            }
            if ring.len() > 4096 {
                ring.drain(..ring.len() - 1024);
            }
            if got {
                silent_sent = false;
                each(&ring[ring.len().saturating_sub(1024)..]);
            } else if !silent_sent {
                ring.clear();
                each(&[]);
                silent_sent = true;
            }
        }
        let _ = client.Stop();
        Ok(())
    }
}
