pub fn cpu_times() -> (u64, u64) {
    let stat = std::fs::read_to_string("/proc/stat").unwrap_or_default();
    let v: Vec<u64> = stat
        .lines()
        .next()
        .unwrap_or_default()
        .split_whitespace()
        .skip(1)
        .filter_map(|n| n.parse().ok())
        .collect();
    let idle = v.get(3).copied().unwrap_or(0) + v.get(4).copied().unwrap_or(0);
    (idle, v.iter().sum::<u64>().saturating_sub(idle))
}

pub fn own_cpu_time() -> u64 {
    0
}

pub fn is_sarab(pid: u32) -> bool {
    std::fs::read_link(format!("/proc/{pid}/exe"))
        .is_ok_and(|p| p.file_name().is_some_and(|n| n == "sarab"))
}
