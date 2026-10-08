use super::*;

#[test]
fn proc_readings() {
    let (idle, busy) = cpu_times();
    assert!(idle + busy > 0, "/proc/stat has CPU time");
    assert!(total_ram() > 0, "/proc/meminfo has MemTotal");
    let (avail, total) = memory();
    assert!(
        avail <= total,
        "memory() is (available, total), as on Windows"
    );
    assert!(!cpu_name().is_empty(), "/proc/cpuinfo has a model name");
    assert!(!is_sarab(std::process::id()), "a test binary is not sarab");
}
