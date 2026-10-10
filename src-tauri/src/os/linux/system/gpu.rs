pub struct GpuMeter;

impl GpuMeter {
    pub fn new() -> Option<GpuMeter> {
        Some(GpuMeter)
    }

    pub fn percent(&mut self) -> Option<f64> {
        std::fs::read_dir("/sys/class/drm")
            .ok()?
            .flatten()
            .filter_map(|e| {
                std::fs::read_to_string(e.path().join("device/gpu_busy_percent"))
                    .ok()?
                    .trim()
                    .parse::<f64>()
                    .ok()
            })
            .reduce(f64::max)
    }
}

pub fn virtual_machine() -> bool {
    static VM: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *VM.get_or_init(|| {
        let read = |f: &str| std::fs::read_to_string(format!("/sys/class/dmi/id/{f}")).unwrap_or_default();
        crate::core::pause::is_vm_vendor(&format!("{} {}", read("sys_vendor"), read("product_name")))
    })
}
