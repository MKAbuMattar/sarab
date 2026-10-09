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
    assert!(!cpu_name().is_empty(), "a CPU name, on x64 and ARM alike");
    assert!(!is_sarab(std::process::id()), "a test binary is not sarab");
}

#[test]
fn cpu_name_on_x86_and_arm() {
    let x86 = "processor	: 0
vendor_id	: AuthenticAMD
model name	: AMD EPYC 7763 64-Core Processor
";
    assert_eq!(
        cpu_name_in(x86).as_deref(),
        Some("AMD EPYC 7763 64-Core Processor")
    );
    let pi = "processor	: 0
BogoMIPS	: 108.00
CPU implementer	: 0x41

Hardware	: BCM2835
Model		: Raspberry Pi 4 Model B Rev 1.4
";
    assert_eq!(
        cpu_name_in(pi).as_deref(),
        Some("Raspberry Pi 4 Model B Rev 1.4")
    );
    let server = "processor	: 0
BogoMIPS	: 50.00
CPU implementer	: 0x41
CPU part	: 0xd0c
";
    assert_eq!(
        cpu_name_in(server),
        None,
        "falls through to the device tree or the CPU type"
    );
}
