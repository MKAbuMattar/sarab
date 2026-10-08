fn meminfo(key: &str) -> u64 {
    std::fs::read_to_string("/proc/meminfo")
        .unwrap_or_default()
        .lines()
        .find_map(|l| {
            l.strip_prefix(key)?
                .trim()
                .strip_suffix("kB")?
                .trim()
                .parse::<u64>()
                .ok()
        })
        .map_or(0, |kb| kb * 1024)
}

pub fn total_ram() -> u64 {
    meminfo("MemTotal:")
}

pub fn memory() -> (u64, u64) {
    (meminfo("MemAvailable:"), total_ram())
}

pub fn net_octets() -> (u64, u64) {
    std::fs::read_to_string("/proc/net/dev")
        .unwrap_or_default()
        .lines()
        .skip(2)
        .filter_map(|l| {
            let (name, rest) = l.split_once(':')?;
            if name.trim() == "lo" {
                return None;
            }
            let v: Vec<u64> = rest
                .split_whitespace()
                .filter_map(|n| n.parse().ok())
                .collect();
            Some((*v.first()?, *v.get(8)?))
        })
        .fold((0, 0), |(a, b), (rx, tx)| (a + rx, b + tx))
}

pub fn cpu_name() -> String {
    std::fs::read_to_string("/proc/cpuinfo")
        .unwrap_or_default()
        .lines()
        .find_map(|l| {
            let (k, v) = l.split_once(':')?;
            (k.trim() == "model name").then(|| v.trim().to_string())
        })
        .unwrap_or_default()
}

pub fn gpu_name() -> String {
    String::new()
}
