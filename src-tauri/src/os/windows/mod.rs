//! Windows backend: WorkerW embedding, pause probes, OS picture wallpaper.

use crate::core::pause::Signals;
use windows::core::{w, BOOL, HSTRING, PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS,
};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, MapWindowPoints, MonitorFromWindow, HDC, HMONITOR,
    MONITORINFOEXW, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
use windows::Win32::System::StationsAndDesktops::{
    CloseDesktop, OpenInputDesktop, DESKTOP_CONTROL_FLAGS, DESKTOP_SWITCHDESKTOP,
};
use windows::Win32::System::Threading::{
    GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Shell::{
    DesktopWallpaper, IDesktopWallpaper, SHQueryUserNotificationState, QUNS_PRESENTATION_MODE,
    QUNS_RUNNING_D3D_FULL_SCREEN,
};
use windows::Win32::UI::WindowsAndMessaging::*;

use windows::Win32::Graphics::Imaging::{IWICBitmapSource, IWICImagingFactory};

mod app_process;
pub use app_process::*;
mod audio;
pub use audio::*;
mod desktop;
pub use desktop::*;
mod dialogs;
pub use dialogs::*;
mod displays;
pub use displays::*;
mod images;
pub use images::*;
mod media;
pub use media::*;
mod mouse;
pub use mouse::*;
mod picture;
pub use picture::*;
mod probes;
pub use probes::*;
mod processes;
pub use processes::*;
mod recycle;
pub use recycle::*;
mod sysinfo;
pub use sysinfo::*;
mod terminal;
pub use terminal::*;
mod toast;
pub use toast::*;

#[cfg(test)]
mod tests;
