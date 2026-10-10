use super::*;

pub fn memory() -> (u64, u64) {
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut m = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    unsafe { GlobalMemoryStatusEx(&mut m) }.map_or((0, 0), |_| (m.ullAvailPhys, m.ullTotalPhys))
}

pub fn net_octets() -> (u64, u64) {
    use windows::Win32::NetworkManagement::IpHelper::{FreeMibTable, GetIfTable2, MIB_IF_TABLE2};
    use windows::Win32::NetworkManagement::Ndis::IfOperStatusUp;
    let mut table: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
    if unsafe { GetIfTable2(&mut table) }.is_err() || table.is_null() {
        return (0, 0);
    }
    let rows = unsafe {
        std::slice::from_raw_parts((*table).Table.as_ptr(), (*table).NumEntries as usize)
    };
    const LOOPBACK: u32 = 24;
    let (mut down, mut up) = (0, 0);
    for r in rows {
        let hardware = r.InterfaceAndOperStatusFlags._bitfield & 1 != 0;
        if hardware && r.Type != LOOPBACK && r.OperStatus == IfOperStatusUp {
            down += r.InOctets;
            up += r.OutOctets;
        }
    }
    unsafe { FreeMibTable(table as *const _) };
    (down, up)
}

pub fn cpu_name() -> String {
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ};
    let mut buf = [0u16; 128];
    let mut len = (buf.len() * 2) as u32;
    let ok = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            w!(r"HARDWARE\DESCRIPTION\System\CentralProcessor\0"),
            w!("ProcessorNameString"),
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
    String::from_utf16_lossy(&buf[..n]).trim().to_string()
}

pub fn gpu_name() -> String {
    use windows::Win32::Graphics::Gdi::{
        EnumDisplayDevicesW, DISPLAY_DEVICEW, DISPLAY_DEVICE_PRIMARY_DEVICE,
    };
    let mut i = 0;
    loop {
        let mut d = DISPLAY_DEVICEW {
            cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32,
            ..Default::default()
        };
        if !unsafe { EnumDisplayDevicesW(PCWSTR::null(), i, &mut d, 0) }.as_bool() {
            return String::new();
        }
        if d.StateFlags.contains(DISPLAY_DEVICE_PRIMARY_DEVICE) {
            let n = d
                .DeviceString
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(d.DeviceString.len());
            return String::from_utf16_lossy(&d.DeviceString[..n]);
        }
        i += 1;
    }
}

pub fn total_ram() -> u64 {
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut m = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    unsafe { GlobalMemoryStatusEx(&mut m) }.map_or(0, |_| m.ullTotalPhys)
}

pub fn local_minutes() -> u32 {
    let t = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
    u32::from(t.wHour) * 60 + u32::from(t.wMinute)
}
