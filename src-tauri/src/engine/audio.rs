//! The audio feed for web wallpapers that ask for it ("api": ["audio"]): what the PC plays,
//! captured by WASAPI loopback and sent as 128 frequency bins about 30 times a second.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub const BINS: usize = 128;
/// Samples per spectrum: about 21 ms at 48 kHz.
const N: usize = 1024;

/// In-place radix-2 FFT over real and imaginary parts; `re.len()` must be a power of two.
fn fft(re: &mut [f32], im: &mut [f32]) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let ang = -2.0 * std::f32::consts::PI / len as f32;
        for start in (0..n).step_by(len) {
            for k in 0..len / 2 {
                let (s, c) = (ang * k as f32).sin_cos();
                let (a, b) = (start + k, start + k + len / 2);
                let (tr, ti) = (re[b] * c - im[b] * s, re[b] * s + im[b] * c);
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
            }
        }
        len <<= 1;
    }
}

/// 128 levels from 0 to 1 for the last `N` mono samples. Bins cover the lower quarter of the
/// FFT (0 to 12 kHz at 48 kHz), two FFT bins each, where music has its energy.
pub fn spectrum(samples: &[f32]) -> [f32; BINS] {
    let mut re = [0f32; N];
    let mut im = [0f32; N];
    let take = samples.len().min(N);
    for (i, s) in samples[samples.len() - take..].iter().enumerate() {
        // Hann window, so a tone between bins does not smear across the whole spectrum.
        let w = 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / (N - 1) as f32).cos();
        re[i] = s * w;
    }
    fft(&mut re, &mut im);
    let mut out = [0f32; BINS];
    for (b, o) in out.iter_mut().enumerate() {
        let m = (0..2)
            .map(|k| {
                let i = b * 2 + k;
                (re[i] * re[i] + im[i] * im[i]).sqrt()
            })
            .fold(0f32, f32::max);
        // A full-scale tone reaches about N/4 after the window; a log scale fits music better.
        *o = ((1.0 + m).ln() / (1.0 + N as f32 / 4.0).ln()).clamp(0.0, 1.0);
    }
    out
}

/// Captures on its own thread while `wanted`.
pub struct Feed {
    pub wanted: Arc<AtomicBool>,
}

impl Feed {
    /// Start capturing. `send` runs on the capture thread with each new spectrum.
    pub fn start(send: impl Fn(&[f32; BINS]) + Send + 'static) -> Feed {
        let wanted = Arc::new(AtomicBool::new(true));
        let w = wanted.clone();
        std::thread::spawn(move || {
            if let Err(e) = crate::os::windows::loopback(&w, &mut |samples: &[f32]| {
                send(&spectrum(samples));
            }) {
                crate::engine::wallpaper::log(format!("audio feed: {e}"));
            }
            w.store(false, Ordering::Relaxed);
        });
        Feed { wanted }
    }

    pub fn running(&self) -> bool {
        self.wanted.load(Ordering::Relaxed)
    }

    pub fn stop(&self) {
        self.wanted.store(false, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spectrum_finds_the_tone() {
        let rate = 48_000.0;
        let tone = |hz: f32| -> Vec<f32> {
            (0..N)
                .map(|i| 0.8 * (2.0 * std::f32::consts::PI * hz * i as f32 / rate).sin())
                .collect()
        };
        let peak = |s: &[f32; BINS]| {
            s.iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .unwrap()
                .0
        };
        // 3 kHz is FFT bin 64 of 1024 at 48 kHz, so output bin 32.
        let s = spectrum(&tone(3000.0));
        assert_eq!(peak(&s), 32);
        assert!(s[32] > 0.8, "a loud tone is near the top: {}", s[32]);
        assert!(s[100] < 0.3, "far bins stay low: {}", s[100]);
        assert_eq!(peak(&spectrum(&tone(9000.0))), 96);
        let silent = spectrum(&[0.0; N]);
        assert!(silent.iter().all(|&v| v == 0.0));
        assert!(spectrum(&[]).iter().all(|&v| v == 0.0));
    }
}
