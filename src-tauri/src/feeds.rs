//! Data web wallpapers can ask for in sarab.json ("api": ["system"]). Nothing here runs unless a
//! playing wallpaper asked, so a wallpaper that does not use it costs nothing.

use crate::os::windows as os;
use serde::Serialize;
use std::time::Instant;

/// What `sarabSystemInfo(info)` receives, once a second.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct SystemInfo {
    pub name_cpu: String,
    pub name_gpu: String,
    /// Percent of all cores, 0 to 100.
    pub current_cpu: f64,
    /// Megabytes.
    pub current_ram_avail: u64,
    pub total_ram: u64,
    /// Bytes per second.
    pub current_net_down: u64,
    pub current_net_up: u64,
}

/// Turns counters since boot into rates between two samples.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sysinfo_reads_real_numbers() {
        let mut s = Sampler::new();
        // Keep a core busy so the CPU figure has something to show.
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
