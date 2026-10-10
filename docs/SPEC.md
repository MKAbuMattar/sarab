# Sarab: specification

> **Sarab** (سراب, "mirage"). A cross-platform animated wallpaper engine built to run on old and low-power machines. Identity: [§9](#9-identity).

Companion docs: [PLAN.md](PLAN.md) (phases, order of work) · [SYSTEM_DESIGN.md](SYSTEM_DESIGN.md) (how it is built).

---

## 1. Design choices

The decisions that shape everything below, and why.

| Choice | Instead of | Why |
|---|---|---|
| One process: tray, core, and a settings window opened on demand | A separate UI process talking over RPC | Fewer processes, no RPC layer, nothing running for a closed window |
| The system webview (WebView2, WKWebView, WebKitGTK) for web pages, video and GIF | Bundling a browser engine and several video players | The system webview already decodes video in hardware on Windows and macOS, and ships with the OS |
| Pictures through the OS wallpaper API | A window that shows a still image | A still image should cost nothing |
| Wallpaper windows inside the main process | A player executable per wallpaper | The webview renderer already runs out of process, so a crashing page cannot take Sarab down; less memory |
| Freeze in place when paused: animation frames stopped, CSS animations and media paused, a pause event sent to the page | Hiding or suspending the wallpaper window | The wallpaper never disappears, so the plain OS wallpaper never shows through; frozen already measured near zero CPU |
| OS job objects and parent-death signals for child processes | A watchdog process | The OS cleans up children for us |
| A local library with drag and drop and `.zip` import | An online gallery with accounts | Needs no server |
| Sarab's own package format (`sarab.json`, `properties.json`) | Reading other apps' formats | One format to keep stable |

---

## 2. Goals

1. Runs on **Windows 10 (1903+) / 11**, **macOS 12+**, and **Linux** (Ubuntu, Debian, Fedora, Arch and derivatives; X11 and Wayland).
2. Plays **HTML pages, URLs, videos, GIFs, pictures**, and (later) native apps/games as the desktop background.
3. **Cheap when idle, cheap when playing.** Usable on a 2015-era dual-core laptop with integrated graphics.
4. **Manual control first**: the user decides what plays and when. Nothing auto-downloads, nothing phones home.
5. Easy to maintain: one language, one repo, one binary per platform.

## 3. Non-goals

- Other apps' wallpaper formats, including proprietary scene formats.
- Online gallery, accounts, workshop.
- Mobile.
- A wallpaper editor.

---

## 4. Platform support matrix

| Platform | Desktop surface | Status target |
|---|---|---|
| Windows 10 1903+ / 11 (incl. 24H2) | `WorkerW` / `Progman` child | Full |
| macOS 12+ (Intel + Apple Silicon) | Borderless window at desktop level on every Space | Full except now-playing |
| Linux Wayland, wlroots (Sway, Hyprland, river, niri, labwc), KDE Plasma 6, COSMIC | `wlr-layer-shell` background layer | Full |
| Linux Wayland, GNOME (Ubuntu default) | Bundled GNOME Shell extension moves our window into the background group | Full, needs the extension enabled |
| Linux X11 (any EWMH window manager) | `_NET_WM_WINDOW_TYPE_DESKTOP`, below, sticky | Best effort. DEs that draw their own desktop window (KDE X11, XFCE) may cover it; picture fallback still works |
| Anything else | Static picture via OS API | Fallback |

Packages: Windows `.msi` + portable `.exe`; macOS `.dmg` (signed + notarized); Linux `.deb`, `.rpm`, `.AppImage`, AUR `PKGBUILD`. Flatpak later.

---

## 5. Wallpaper types

| Type | Input | Renderer | Interactive | Notes |
|---|---|---|---|---|
| `picture` | jpg, png, bmp, webp, (heic on macOS) | OS wallpaper API | No | Zero runtime cost |
| `video` | mp4, webm, mkv*, mov, m4v | Webview `<video>` (Win, mac); webview or mpv (Linux) | No | Muted by default, loops. *mkv/HEVC depend on OS codecs |
| `gif` | gif, animated webp/png | Webview `<img>` | No | Offer "convert to video" on import when ffmpeg is present (a GIF costs more CPU than the same clip as H.264) |
| `web` | folder with `index.html` | Webview, served from the asset protocol scoped to that folder | Yes | Sarab packages |
| `webaudio` | same as `web` | Webview + audio capture | Yes | Audio only captured while playing |
| `url` | http(s) link, Shadertoy, YouTube | Webview | Yes | Shadertoy/YouTube links rewritten to embed/fullscreen form |
| `app` | exe / binary (Unity, Godot) | Reparented native window | Yes | Windows + X11 only. Phase 5 |

Sarab has its own package format (`sarab.json` + optional `properties.json`, see §6.4). It reads no other app's format.

---

## 6. Functional requirements

### 6.1 Library
- F1. Library folder (default: `<data dir>/Library`), one subfolder per wallpaper, each with `sarab.json`.
- F2. Add by drag-and-drop, file picker, or pasting a URL. Sarab creates the folder and `sarab.json`.
- F3. Import Sarab package `.zip` files (`sarab.json` at the root). Reject zip entries that escape the target folder.
- F4. Export any wallpaper as a Sarab package `.zip`.
- F5. Thumbnail: use the package's own, else capture one frame on first play.
- F6. Delete, rename, edit title/description/tags.

### 6.2 Apply and layout
- F7. Apply a wallpaper to one display, to all displays (duplicate), or across displays (span).
- F8. Layout persists and restores on login and after a crash.
- F9. Hot-plug: display added → apply saved or default layout; display removed → close its wallpaper.
- F10. Scaling modes for video/GIF/picture: fill, fit, stretch, center (`object-fit` in the webview).
- F11. Autostart on login (off by default, toggle in settings).
- F12. Close wallpaper and restore the previous OS wallpaper.

### 6.3 Pause and performance (the core requirement)
Each rule is **pause**, **ignore**, or (for fullscreen/maximized) **per display**.

| ID | Trigger | Default |
|---|---|---|
| P1 | A fullscreen app on that display (game, video player, presentation) | Pause |
| P2 | A maximized window covering that display | Pause |
| P3 | Any focused app (not the desktop) | Ignore |
| P4 | On battery | Pause |
| P5 | OS energy-saver / low-power mode on | Pause |
| P6 | Screen locked or display asleep | Pause |
| P7 | Remote desktop session (Windows) | Pause |
| P8 | Per-app rule: process name → always pause / always play | Empty list |
| P9 | Manual pause from tray or CLI (wins over everything) |, |

- F13. Paused **web** wallpaper: frozen on its last frame and never hidden (hiding shows the plain Windows wallpaper), media paused, `sarabPlaybackChanged({paused: true})` fired.
- F14. Paused **video**: `video.pause()`. On Linux mpv: `set pause yes`.
- F15. **FPS cap** for web wallpapers: 15 / 30 / 60 / unlimited (default 30). Implemented by throttling `requestAnimationFrame`.
- F16. **Low-power profile** (one toggle, auto-on when on battery if chosen): FPS cap 15, audio visualizer off, system-info polling off.
- F17. When **all** displays are paused for more than 5 minutes, drop the web content (navigate to `about:blank`) to free RAM, and reload on resume. Configurable; off on machines with ≥ 8 GB RAM.
- F18. Audio: muted by default; global volume; "audio only on desktop" (mute when any window is focused).

### 6.4 Package format and web wallpaper API
The host calls these functions if the page defines them:

| Function | Payload | When |
|---|---|---|
| `sarabPropertyChanged(name, value)` | property key, new value | On load (all props) and on each change |
| `sarabPlaybackChanged(state)` | `{paused: bool}` | Pause/resume |
| `sarabAudio(arr)` (Phase 4) | 128 floats, 0..1 | ~30 Hz while playing, `webaudio` only |
| `sarabSystemInfo(info)` (Phase 4) | `NameCpu, NameGpu, CurrentCpu, CurrentRamAvail, TotalRam, CurrentNetDown, CurrentNetUp` | 1 Hz, only if the package opts in |
| `sarabNowPlaying(track)` (Phase 4) | `Title, Artist, AlbumTitle, AlbumArtist, Thumbnail (base64), State, Position, Duration` | On track or playback change and every 10 s, only if the package opts in. Wallpaper Engine's `wallpaperRegisterMedia*Listener` functions get the same data |


`properties.json` controls: `slider`, `textbox`, `dropdown`, `folderDropdown`, `button`, `label`, `color`, `checkbox`. The settings UI renders these; changed values are saved per wallpaper per display.

`sarab.json` fields: `title`, `description`, `author`, `license`, `type` (`web`, `url`, `video`, `gif`, `picture`, `app`), `file`, `external`, `thumbnail`, `tags`, `version`.

### 6.5 Interaction
- F19. Mouse position and clicks reach interactive wallpapers when the desktop is under the cursor. Off by default (it costs a global input hook). Keyboard forwarding: Windows only, opt-in.
- F20. Wayland: the background surface receives pointer input natively; no hook needed.

### 6.6 Control
- F21. Tray icon: pause/resume all, next wallpaper, open library, quit.
- F22. CLI (same binary, forwarded to the running instance):
  `sarab set <path|url|id> [--display N]` · `sarab close [--display N]` · `sarab pause|play|resume|toggle` · `sarab prop <key>=<value> [--display N]` · `sarab volume <0-100>` · `sarab next` · `sarab import <zip>` · `sarab ui` · `sarab status` · `sarab quit`
- F23. Playlist/rotation: change wallpaper every N minutes from a tag or folder (Phase 5).

### 6.7 Screensaver (Phase 5)
- F24. In-app screensaver: after N minutes idle, show a chosen wallpaper fullscreen and topmost on every display; any input dismisses it. Works on all platforms.
- F25. Windows `.scr` shim so it appears in the Windows screensaver settings.

---

## 7. Non-functional requirements

### 7.1 Performance budgets
Reference low-end machine: **Intel Core i3/i5 5th gen (or AMD A-series), 4 GB RAM, integrated GPU, SSD or HDD, 1080p display**. Measured with the OS task manager over 60 s.

| Scenario | CPU (whole app) | GPU | RAM (app + webview) |
|---|---|---|---|
| Picture wallpaper | 0% | 0% | < 30 MB |
| Any wallpaper, paused | < 0.5% | ~0% | web ≤ 120 MB, or < 40 MB after F17 unloads it |
| 1080p H.264 video, hardware decode | < 5% | < 15% | < 150 MB |
| Typical web/canvas wallpaper at 30 FPS cap | < 10% | < 25% | < 200 MB |
| Pause reaction after fullscreen app starts | ≤ 1 s | | |
| Cold start to wallpaper visible | ≤ 2 s (SSD) | | |
| Installer / package size | ≤ 15 MB (excluding OS webview runtime) | | |

A release cannot ship if the paused or picture budgets are missed.

### 7.2 Reliability
- R1. If Sarab crashes or is killed, the desktop returns to the normal OS wallpaper and no child processes are left. On Windows a stale frame can remain until the desktop repaints; the next start forces that repaint.
- R2. Survives Explorer restart (Windows), Dock/Finder restart (macOS), compositor/shell restart (Linux) by re-embedding.
- R3. A wallpaper that fails to load shows an error in the UI and falls back to the previous wallpaper. It does not crash the app.
- R4. Settings writes are atomic (write temp file, rename).

### 7.3 Security
- S1. Web wallpapers are untrusted code. They get **no** access to the app's IPC/commands. Only the one-way host→page API above.
- S2. Local wallpapers are served through the app's asset protocol, whose scope is opened at runtime only for the wallpapers in use. No `file://`, no path traversal.
- S3. Zip import rejects absolute paths and `..` entries, caps total unpacked size (default 2 GB).
- S4. `app` wallpapers run native code: explicit confirmation on first use, shows the full path.
- S5. No telemetry. Network use: only what a wallpaper itself loads, plus an update check the user can turn off.

### 7.4 Accessibility and UX
- Settings UI is keyboard navigable, respects OS dark/light theme and reduced-motion (reduced-motion on → default to paused-on-start).
- All strings in one JSON file per language. English and Arabic (RTL) from v0.1 (see §9).

---

## 8. Acceptance tests (manual, per release, per platform)

1. Set each wallpaper type; verify it renders under the desktop icons and icons stay clickable.
2. Launch a fullscreen game/video player → wallpaper pauses within 1 s; exit → resumes.
3. Unplug the charger with P4 on → pauses. Plug in → resumes.
4. Lock the screen → pauses. Unlock → resumes.
5. Kill the process from the task manager → desktop shows the normal wallpaper; no child processes left.
6. Restart Explorer / Finder / the compositor → wallpaper comes back within 5 s.
7. Two displays, hot-unplug one → no crash, remaining display unaffected.
8. Import 3 Sarab packages with `properties.json` → properties UI works, changes apply live.
9. Measure the budgets in 7.1 on the reference machine and record the numbers in the release notes.

---

## 9. Identity

### Name
**Sarab**, سراب, "mirage". The shimmer over Jordan's desert that looks alive but isn't there: an image that moves behind everything else. That's what a live wallpaper is.

| Use | Form |
|---|---|
| Product name | Sarab (Arabic: سراب) |
| Binary, CLI, package names | `sarab` (AUR: `sarab-bin`) |
| App ID | `com.mkabumattar.sarab` (not `.app`, which clashes with macOS bundles; switch to your own reverse domain before a public release) |
| Tagline | EN: "A living desktop that rests when you work." · AR: "خلفية حيّة ترتاح حين تعمل." |

Check the name for trademark and package-name clashes (Microsoft Store, Mac App Store, AUR, Flathub) before the first public release.

### Palette
Source: [Jordanian Identity Colors](https://raw.githubusercontent.com/MKAbuMattar/black-iris/refs/heads/main/.github/assets/jordan-identity-colors.json) (10 tones).

| Token | Color | Hex | Role |
|---|---|---|---|
| `iris` | Black Iris · السوسنة السوداء | `#2B2233` | Brand primary. Dark background, light-theme text |
| `sand` | Wadi Rum Sand · رمال وادي رم | `#D9A36A` | Brand accent. Dark-theme accent, logo mark |
| `salt` | Salt White · ملح الأردن | `#EAE6DB` | Light background, dark-theme text |
| `amman` | Amman Stone · حجر عمان القديم | `#D9C9B0` | Light surfaces, dark-theme secondary text |
| `basalt` | Basalt Black · بازلت الحرة | `#3C3C3C` | Dark surfaces, light-theme secondary text |
| `deadsea` | Dead Sea Blue · أزرق البحر الميت | `#2F5D6B` | Light-theme accent, primary buttons, links |
| `keffiyeh` | Keffiyeh Red · أحمر الشماغ | `#8B1E2D` | Danger (delete, errors) |
| `petra` | Petra Rose · وردي البتراء | `#C76B6B` | Dark-theme danger icons/borders, "paused" badge |
| `olive` | Olive Green · زيتون أردني | `#6B7A4F` | Success / "playing" indicator (icons and borders, not body text) |
| `camel` | Desert Camel · لون الهجن | `#B9875E` | Illustration, hover states, charts |

### Theme tokens
Contrast ratios below were computed with the WCAG 2 formula. Body text needs ≥ 4.5, large text and icons ≥ 3.

```css
:root {                         /* light */
  --bg: #EAE6DB;                /* salt   */
  --surface: #D9C9B0;           /* amman  */
  --text: #2B2233;              /* iris    12.2 on bg · 9.4 on surface */
  --text-muted: #3C3C3C;        /* basalt   8.9 on bg · 6.8 on surface */
  --accent: #2F5D6B;            /* deadsea  5.8 on bg · 4.5 on surface */
  --on-accent: #EAE6DB;         /* salt on deadsea 5.8 */
  --danger: #8B1E2D;            /* keffiyeh 7.3 on bg */
  --success: #6B7A4F;           /* olive    3.7 on bg: icons only */
  --highlight: #D9A36A;         /* sand: decoration only, 1.8 on bg */
}
@media (prefers-color-scheme: dark) {
  :root {
    --bg: #2B2233;              /* iris   */
    --surface: #3C3C3C;         /* basalt */
    --text: #EAE6DB;            /* salt    12.2 on bg · 8.9 on surface */
    --text-muted: #D9C9B0;      /* amman    9.4 on bg · 6.8 on surface */
    --accent: #D9A36A;          /* sand     6.8 on bg · 4.9 on surface */
    --on-accent: #2B2233;       /* iris on sand 6.8 */
    --danger: #C76B6B;          /* petra    4.2 on bg: icons, large text; danger buttons use keffiyeh fill + salt text (7.3) */
    --success: #6B7A4F;         /* olive    3.3 on bg: icons only */
    --highlight: #B9875E;       /* camel */
  }
}
```

Pairs that fail and must not carry text: sand on salt (1.8), olive on anything for body text, keffiyeh on iris (1.7), deadsea on iris (2.1), amman on salt (1.3).

### Logo and icons
- Mark: a single black iris flower drawn in `sand` on an `iris` rounded square. The petals fading at their tips reads as a heat mirage. One solid color, so it works as a 16 px tray icon.
- Tray icon: monochrome version (white on Windows/Linux dark trays, template image on macOS). A small `petra` dot marks "paused".
- App icon sizes: generate with `cargo tauri icon` from one 1024 px PNG.

### Language
- UI ships in English and Arabic from v0.1. Arabic uses full RTL layout (`dir="rtl"` on `<html>`, CSS logical properties such as `margin-inline-start` instead of `margin-left`).
- Arabic UI font: the system font (Segoe UI on Windows, SF Arabic on macOS, Noto Sans Arabic on Linux). No bundled font files.
