//! The running app: commands, the settings window, the tray, and what ties them to the core.

use crate::core::{settings, support, update};
use crate::engine::wallpaper;
use crate::library::presets;
use crate::{cli, library, os};

use crate::cli::Command;
use serde_json::{json, Value};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_autostart::ManagerExt;
use wallpaper::{log, on_main, Core, Shared};

mod commands;
pub(crate) use commands::*;
mod dispatch;
pub(crate) use dispatch::*;
mod i18n;
pub(crate) use i18n::*;
mod tray;
pub(crate) use tray::*;
mod window;
pub(crate) use window::*;
