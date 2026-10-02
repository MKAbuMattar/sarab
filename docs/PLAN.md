# Sarab: plan

Companion docs: [SPEC.md](SPEC.md) (what it must do) · [SYSTEM_DESIGN.md](SYSTEM_DESIGN.md) (how it is built).

**One-line summary:** a Rust + Tauri v2 app that places a system-webview window under the desktop icons on Windows, macOS and Linux, plays HTML / video / GIF in it, uses the OS wallpaper API for pictures, and pauses aggressively so it stays near 0% CPU on old hardware.

Estimates assume one developer who knows Rust, working full days. Each phase ends with a **gate**: a check that must pass before the next phase starts.

---

## Phase 0: spikes (≈ 1 week)

Goal: prove the three risky parts before writing the real app. Throwaway code, one small Tauri project.

- [ ] **0.1 Windows embed.** Tauri window with a red page, parented to WorkerW. Test on Windows 10 and on Windows 11 24H2 (raised desktop). Icons must stay visible and clickable.
- [ ] **0.2 Wayland layer-shell.** Same red window as a `Background` layer on Hyprland or Sway, and on KDE Plasma 6. If Tauri maps the window before layer-shell init, try the fallback (raw `gtk::Window` + `wry::build_gtk`).
- [ ] **0.3 GNOME.** Minimal extension that moves a window with `wm_class=sarab-wallpaper` into the background group on Ubuntu 24.04.
- [ ] **0.4 macOS.** Desktop-level window on all Spaces, Finder icons still clickable.
- [ ] **0.5 X11.** `_NET_WM_WINDOW_TYPE_DESKTOP` window on one plain WM (i3 or Openbox) and on Ubuntu Xorg.
- [ ] **0.6 Video cost.** Loop a 1080p H.264 mp4 in `<video>` on the reference low-end machine (SPEC §7.1), on each OS. Record CPU / GPU / RAM. Record the same for a paused video and a hidden window.

**Gate 0:** a results table in `docs/spikes.md` with a yes/no per platform and the measured numbers.
- Linux video over budget → add the mpv backend to Phase 2.
- Layer-shell needs the fallback → note it in SYSTEM_DESIGN §6.

---

## Phase 1: Windows MVP (≈ 3 weeks)

Goal: something usable every day on Windows. Windows first because the reference project and the owner's machine are on Windows.

- [ ] 1.1 Project skeleton: repo layout from SYSTEM_DESIGN §3, `tauri.conf.json`, capabilities (commands for the `main` window only), CI build for Windows.
- [ ] 1.2 `settings.rs`: `Settings` + `Layout` structs, atomic save, defaults.
- [ ] 1.3 `library.rs`: scan the library folder, read `sarab.json`, add file/folder/URL, import Sarab `.zip` (zip-slip test), delete.
- [ ] 1.4 `os/windows.rs`: WorkerW attach (classic + 24H2), Explorer-restart re-attach, desktop repaint on exit/start, `IDesktopWallpaper` for pictures.
- [ ] 1.5 `wallpaper.rs`: apply/close per display, `ui/player.html` for video/GIF, asset-protocol scope opened per wallpaper, `call()` bridge, `inject.js` (FPS cap, freeze).
- [ ] 1.6 `pause.rs`: `decide()` + its unit test table. Windows probes: fullscreen (`SHQueryUserNotificationState` + rect check), battery, energy saver, lock, display sleep, remote session, foreground app name.
- [x] 1.7 Covered pause on Windows: freeze in place. (Hide + `TrySuspend()` was tried and dropped: it showed the Windows wallpaper.)
- [x] 1.8 `properties.json`: render controls in the settings UI, save per display, push with `sarabPropertyChanged`.
- [ ] 1.9 Tray: pause/resume, open library, quit. Autostart toggle. Single instance.
- [ ] 1.10 Settings/library UI in plain HTML (library grid, drag-and-drop, per-wallpaper properties, pause rules, FPS cap, volume). Theme tokens and RTL from SPEC §9; English + Arabic strings.
- [ ] 1.11 Multi-monitor: per display + duplicate, hot-plug.

**Gate 1:** SPEC §8 acceptance tests 1 to 8 pass on Windows 10 and Windows 11 24H2. Paused and picture budgets (SPEC §7.1) met on the reference machine. Five Sarab packages with `properties.json` import and run.

---

## Phase 2: Linux (≈ 3 weeks)

- [ ] 2.1 `os/wayland.rs`: layer-shell attach per output, output hot-plug, foreign-toplevel fullscreen/maximized probe.
- [ ] 2.2 `gnome-extension/`: background-group placement + `FullscreenChanged` D-Bus signal. Picture via `gsettings`.
- [ ] 2.3 `os/x11.rs`: desktop-type window, `_NET_WM_STATE` probe via PropertyNotify.
- [ ] 2.4 Linux probes over `zbus`: UPower, power-profiles-daemon, logind lock, ScreenSaver.
- [ ] 2.5 Detect the missing GStreamer VA plugin at start and show a one-line hint with the distro package name.
- [ ] 2.6 (Only if Gate 0 said so) mpv backend: X11 `--wid`, Wayland `mpvpaper`.
- [ ] 2.7 Packaging: `.deb`, `.rpm`, `.AppImage` from CI on `ubuntu-22.04`; AUR `PKGBUILD`.

**Gate 2:** acceptance tests pass on Ubuntu 24.04 GNOME Wayland, Ubuntu Xorg session, Fedora KDE Wayland, Arch + Hyprland. Packages install cleanly on a fresh VM of each.

---

## Phase 3: macOS (≈ 1.5 weeks)

- [ ] 3.1 `os/macos.rs`: desktop-level window per `NSScreen`, screen hot-plug, `ActivationPolicy::Accessory`.
- [ ] 3.2 Probes: occlusion notification, `CGWindowListCopyWindowInfo` per display, IOKit battery, low-power mode, lock/sleep notifications, frontmost app.
- [ ] 3.3 Pictures via `NSWorkspace.setDesktopImageURL`.
- [ ] 3.4 Universal `.dmg`, signing + notarization in CI (needs an Apple Developer account).

**Gate 3:** acceptance tests pass on one Intel Mac and one Apple Silicon Mac; Spaces and fullscreen apps pause correctly.

---

## Phase 4: Wallpaper API and interaction (≈ 2.5 weeks)

- [ ] 4.1 `sarabSystemInfo` via `sysinfo` (`sarabPlaybackChanged` is done).
- [ ] 4.2 Audio feed: Windows WASAPI loopback, Linux Pulse/PipeWire monitor. 128 bins at 30 Hz, only while an audio wallpaper plays.
- [ ] 4.3 Now playing: Windows GSMTC, Linux MPRIS.
- [ ] 4.5 Mouse forwarding (SYSTEM_DESIGN §9) on Windows, macOS, X11. Native on Wayland.
- [ ] 4.6 CLI: `set`, `close`, `pause`, `resume`, `toggle`, `prop`, `volume`, `next`.
- [ ] 4.7 macOS audio via ScreenCaptureKit (optional, needs permission prompt).

**Gate 4:** a Sarab audio visualizer package reacts to music on Windows and Linux. CLI test passes. Budgets from §7.1 still met with feeds off.

---

## Phase 5: extras (pick by demand, ≈ 1 week each)

- [ ] 5.1 Playlists / rotation by folder or tag (every N minutes, or at login).
- [ ] 5.2 Span layout across displays.
- [ ] 5.3 In-app screensaver (idle timer → fullscreen topmost wallpaper) + Windows `.scr` shim.
- [ ] 5.4 App/game wallpapers on Windows (Job Object, Unity `-parentHWND`) and X11.
- [ ] 5.5 Unload-after-long-pause (SPEC F17) and low-power profile auto-switch.
- [ ] 5.6 Export Sarab `.zip`; thumbnail capture.
- [ ] 5.7 Flatpak.
- [ ] 5.8 More translations beyond English and Arabic.

---

## Timeline

| Phase | Weeks | Cumulative |
|---|---|---|
| 0 Spikes | 1 | 1 |
| 1 Windows MVP | 3 | 4 |
| 2 Linux | 3 | 7 |
| 3 macOS | 1.5 | 8.5 |
| 4 API + interaction | 2.5 | 11 |
| 5 Extras | 1 each |, |

A first public release (**v0.1**) is possible after Phase 2: Windows + Linux, core wallpaper types, pause rules. macOS joins in v0.2, the API features in v0.3.

---

## Working rules

1. **Measure before optimizing.** Every performance change lands with before/after numbers from the SPEC §7.1 script.
2. **Paused budget is a release blocker.** If paused CPU is above 0.5%, the release waits.
3. **No new dependency without a line in SYSTEM_DESIGN §12** saying why.
4. **Keep the package format stable.** Any change to `sarab.json` or the JS API keeps old packages loading (add fields, never rename).
5. **Platform code stays in `os/*.rs`.** Everything else compiles unchanged on all three OSes.

---

## First three actions

1. Create the repo and a bare `cargo tauri init` project (`sarab/`), commit.
2. Write spike 0.1 (Windows WorkerW embed) in `os/windows.rs`, following Microsoft's guidance for the raised desktop: a layered child of Progman between `SHELLDLL_DefView` and `WorkerW`.
3. Run spike 0.6 on the oldest laptop available and write the numbers down.
