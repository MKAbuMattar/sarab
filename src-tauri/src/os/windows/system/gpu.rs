use super::*;
use windows::Win32::System::Performance::{
    PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW,
    PdhOpenQueryW, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY,
    PDH_MORE_DATA,
};

pub struct GpuMeter {
    query: PDH_HQUERY,
    counter: PDH_HCOUNTER,
}

unsafe impl Send for GpuMeter {}

impl GpuMeter {
    pub fn new() -> Option<GpuMeter> {
        let mut query = PDH_HQUERY::default();
        let mut counter = PDH_HCOUNTER::default();
        unsafe {
            if PdhOpenQueryW(PCWSTR::null(), 0, &mut query) != 0 {
                return None;
            }
            let path = w!(r"\GPU Engine(*engtype_3D)\Utilization Percentage");
            if PdhAddEnglishCounterW(query, path, 0, &mut counter) != 0
                || PdhCollectQueryData(query) != 0
            {
                let _ = PdhCloseQuery(query);
                return None;
            }
        }
        Some(GpuMeter { query, counter })
    }

    pub fn percent(&mut self) -> Option<f64> {
        unsafe {
            if PdhCollectQueryData(self.query) != 0 {
                return None;
            }
            let (mut size, mut count) = (0u32, 0u32);
            if PdhGetFormattedCounterArrayW(self.counter, PDH_FMT_DOUBLE, &mut size, &mut count, None)
                != PDH_MORE_DATA
            {
                return None;
            }
            let mut buf = vec![0u8; size as usize];
            let items = buf.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W;
            if PdhGetFormattedCounterArrayW(
                self.counter,
                PDH_FMT_DOUBLE,
                &mut size,
                &mut count,
                Some(items),
            ) != 0
            {
                return None;
            }
            let own = own_processes();
            let total: f64 = std::slice::from_raw_parts(items, count as usize)
                .iter()
                .filter(|i| {
                    gpu_instance_pid(&i.szName.to_string().unwrap_or_default())
                        .is_none_or(|pid| !own.contains(&pid))
                })
                .map(|i| i.FmtValue.Anonymous.doubleValue)
                .sum();
            Some(total.min(100.0))
        }
    }
}

impl Drop for GpuMeter {
    fn drop(&mut self) {
        unsafe {
            let _ = PdhCloseQuery(self.query);
        }
    }
}

pub(in crate::os::windows) fn gpu_instance_pid(name: &str) -> Option<u32> {
    name.strip_prefix("pid_")?.split('_').next()?.parse().ok()
}

fn bios(value: PCWSTR) -> String {
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ};
    let mut buf = [0u16; 256];
    let mut len = (buf.len() * 2) as u32;
    let ok = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            w!(r"HARDWARE\DESCRIPTION\System\BIOS"),
            value,
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr() as *mut _),
            Some(&mut len),
        )
    }
    .is_ok();
    let n = if ok {
        (len as usize / 2).saturating_sub(1)
    } else {
        0
    };
    String::from_utf16_lossy(&buf[..n])
}

pub fn virtual_machine() -> bool {
    static VM: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *VM.get_or_init(|| {
        crate::core::pause::is_vm_vendor(&format!(
            "{} {}",
            bios(w!("SystemManufacturer")),
            bios(w!("SystemProductName"))
        ))
    })
}
