use crate::os::platform as os;
use serde::Serialize;
use std::time::Instant;

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct SystemInfo {
    pub name_cpu: String,
    pub name_gpu: String,
    pub current_cpu: f64,
    pub current_ram_avail: u64,
    pub total_ram: u64,
    pub current_net_down: u64,
    pub current_net_up: u64,
}

pub struct Sampler {
    names: (String, String),
    cpu: (u64, u64),
    net: (u64, u64),
    at: Instant,
}

impl Sampler {
    pub fn new() -> Sampler {
        Sampler {
            names: (os::cpu_name(), os::gpu_name()),
            cpu: os::cpu_times(),
            net: os::net_octets(),
            at: Instant::now(),
        }
    }

    pub fn sample(&mut self) -> SystemInfo {
        let (cpu, net, at) = (os::cpu_times(), os::net_octets(), Instant::now());
        let idle = cpu.0.saturating_sub(self.cpu.0);
        let busy = cpu.1.saturating_sub(self.cpu.1);
        let secs = at.duration_since(self.at).as_secs_f64().max(0.001);
        let per_sec = |now: u64, was: u64| (now.saturating_sub(was) as f64 / secs) as u64;
        let (avail, total) = os::memory();
        let info = SystemInfo {
            name_cpu: self.names.0.clone(),
            name_gpu: self.names.1.clone(),
            current_cpu: if idle + busy == 0 {
                0.0
            } else {
                (busy as f64 * 1000.0 / (idle + busy) as f64).round() / 10.0
            },
            current_ram_avail: avail >> 20,
            total_ram: total >> 20,
            current_net_down: per_sec(net.0, self.net.0),
            current_net_up: per_sec(net.1, self.net.1),
        };
        (self.cpu, self.net, self.at) = (cpu, net, at);
        info
    }
}

#[derive(Serialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Track {
    pub title: String,
    pub artist: String,
    pub album_title: String,
    pub album_artist: String,
    pub thumbnail: String,
    pub state: String,
    pub position: f64,
    pub duration: f64,
}

impl Track {
    pub fn same_song(&self, other: &Track) -> bool {
        Track {
            position: 0.0,
            ..self.clone()
        } == Track {
            position: 0.0,
            ..other.clone()
        }
    }
}

fn base64(bytes: &[u8]) -> String {
    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (u32::from(c[0]) << 16)
            | (u32::from(*c.get(1).unwrap_or(&0)) << 8)
            | u32::from(*c.get(2).unwrap_or(&0));
        for (i, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            out.push(if i <= c.len() {
                ABC[(n >> shift) as usize & 63] as char
            } else {
                '='
            });
        }
    }
    out
}

pub fn read_track() -> Option<Track> {
    let m = os::now_playing()?;
    let art = m.art;
    let thumbnail = if art.is_empty() {
        String::new()
    } else {
        let kind = if art.starts_with(b"\x89PNG") {
            "png"
        } else {
            "jpeg"
        };
        format!("data:image/{kind};base64,{}", base64(&art))
    };
    Some(Track {
        title: m.title,
        artist: m.artist,
        album_title: m.album_title,
        album_artist: m.album_artist,
        thumbnail,
        state: m.state.into(),
        position: m.position,
        duration: m.duration,
    })
}

pub struct NowPlaying {
    pub wanted: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub latest: std::sync::Arc<std::sync::Mutex<Option<Track>>>,
    pub sent: Option<Option<Track>>,
}

impl NowPlaying {
    pub fn start() -> NowPlaying {
        use std::sync::atomic::Ordering;
        let wanted = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let latest = std::sync::Arc::new(std::sync::Mutex::new(None));
        let (w, l) = (wanted.clone(), latest.clone());
        std::thread::spawn(move || loop {
            if w.load(Ordering::Relaxed) {
                let t = read_track();
                *l.lock().unwrap() = t;
            }
            std::thread::sleep(std::time::Duration::from_secs(2));
        });
        NowPlaying {
            wanted,
            latest,
            sent: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_playing_shape() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        let json = serde_json::to_value(Track::default()).unwrap();
        for k in [
            "Title",
            "Artist",
            "AlbumTitle",
            "AlbumArtist",
            "Thumbnail",
            "State",
            "Position",
            "Duration",
        ] {
            assert!(json.get(k).is_some(), "{k}");
        }
        let _ = read_track();
        let a = Track {
            title: "A".into(),
            position: 10.0,
            ..Track::default()
        };
        assert!(a.same_song(&Track {
            position: 12.0,
            ..a.clone()
        }));
        assert!(!a.same_song(&Track {
            state: "paused".into(),
            ..a.clone()
        }));
    }

    #[test]
    fn sysinfo_reads_real_numbers() {
        let mut s = Sampler::new();
        let t = Instant::now();
        let mut x = 0u64;
        while t.elapsed().as_millis() < 300 {
            x = x.wrapping_mul(31).wrapping_add(7);
        }
        std::hint::black_box(x);
        let i = s.sample();
        assert!(!i.name_cpu.is_empty(), "cpu name");
        assert!(i.total_ram > 0 && i.current_ram_avail <= i.total_ram);
        assert!((0.0..=100.0).contains(&i.current_cpu), "{}", i.current_cpu);
        assert!(i.current_cpu > 0.0, "a busy core shows up");
        let json = serde_json::to_value(&i).unwrap();
        for k in [
            "NameCpu",
            "NameGpu",
            "CurrentCpu",
            "CurrentRamAvail",
            "TotalRam",
            "CurrentNetDown",
            "CurrentNetUp",
        ] {
            assert!(json.get(k).is_some(), "{k}");
        }
    }
}
