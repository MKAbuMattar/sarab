# Changelog

All notable changes to Sarab are listed here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Changed

- Edit wallpaper has tabs: Details, and Part that plays for videos. A dot marks a tab with unsaved changes, and a failed save opens the tab with the problem.
- Edit wallpaper has a Thumbnail tab: keep the automatic picture or use an image of your own (PNG, JPEG or WebP, up to 10 MB). Your image is copied into the wallpaper's folder when you save, and stays there if you switch back to automatic. It also gives web pages, apps and Linux wallpapers a thumbnail.

### Added

- Edit wallpaper has a "Part that plays" editor for videos: drag the start and end handles, type the times, or press [ and ] while it plays. "Seam only" loops the moment the end runs into the start, the one you see over and over on the desktop. Displays playing the wallpaper switch to the new part when you save.

### Fixed

- Text in dialogs could be selected with the mouse. Chromium's own stylesheet makes modal dialogs selectable; Sarab's dialogs now behave like the rest of the window.

## [0.0.10] - 2026-10-09

### Added

- A first Linux preview, for x64 and ARM64: `.deb`, `.rpm`, AppImage and an Arch package with each release. Wallpapers play on X11 desktops, and on Wayland desktops with the layer-shell protocol (KDE Plasma, Sway, Hyprland, COSMIC and others); on GNOME, Sarab runs through XWayland. Wallpapers do not rest on Linux yet.
- Text boxes have Sarab's own right-click menu: Undo, Cut, Copy, Paste and Select all.
- `sarab help` lists every command, and `sarab help <command>` (or `sarab <command> --help`) shows its synopsis, options and examples.
- `sarab --version` prints one line for bug reports: `sarab/0.0.10 Windows/11 build/26100 WebView2/... exe/x64 channel/stable`.

### Changed

- The title bar's menu (right-click the title bar, Alt+Space) and the tray menu are Sarab's own menus, in its theme and language, instead of the Windows ones.

## [0.0.9] - 2026-10-09

### Added

- Windows installers for 32-bit (x86) and ARM64 PCs, next to x64. The updater installs the one that matches the PC.

### Fixed

- Pausing an app wallpaper could leave threads running that the program started at that moment. Sarab now sweeps until no new thread appears, and resumes exactly the threads it suspended.
- Window styles on 32-bit Windows.

## [0.0.8] - 2026-10-07

### Changed

- App wallpapers rest with the others: when a window covers the display, the PC locks, you pause, or any other pause rule applies, Sarab suspends the program and resumes it when the wallpaper plays again. A game set as a wallpaper no longer uses the GPU behind a maximized window.
- The display map shows a thumbnail of what each display plays.
- About > Website opens [sarab.mkabumattar.com](https://sarab.mkabumattar.com) instead of the GitHub repository.

### Fixed

- Web and web address wallpapers get thumbnails again. Since 0.0.5 every capture failed: WebView2 still held the file when Sarab read it, and two displays playing one wallpaper wrote the same file.
- An app wallpaper starts suspended and runs only once it is in its job, so a program it starts in its first instant also ends when the wallpaper changes or Sarab quits.

## [0.0.7] - 2026-10-07

### Added

- `sarab status` prints in the terminal: Sarab's version, then one line per display with the wallpaper's title and whether it plays or why it rests. `--json` gives the raw status for scripts, and it says so when Sarab is not running. The installer adds `sarab.com` next to `sarab.exe` so the terminal waits for the answer.
- Import Wallpaper Engine web, video and application wallpapers: `sarab set` or the Add box takes the folder or its `project.json`. Their settings become Customize controls (sliders, checkboxes, colors, lists, text), and pages that listen for properties, audio or pause work as they do there. Scene wallpapers need Wallpaper Engine's own renderer and are refused with a clear message.

### Changed

- The source is grouped into folders by role, one job per file, without comments.

## [0.0.6] - 2026-10-06

### Added

- Hovering a video or GIF tile plays it there, muted; it stops when the pointer leaves, and never plays when Windows asks for less motion.
- Keep the last frame when Sarab quits (Settings > Wallpapers, off by default): each display keeps the moment it showed as its Windows wallpaper.
- `sarab screenshot <full path.png> [--display N]` saves what a display shows as a PNG.
- Packages can carry their title and description in other languages (`titles` and `descriptions` in sarab.json); the window shows the one for its language, and search and sorting use it.
- Customize has two more control types for packages: number (with min, max and step) and password (shown as dots).
- A package made for a newer Sarab says so on its tile and asks before it plays. Exported packages record the version that made them.
- Pause while other apps keep the CPU over 70, 80 or 90 percent (Settings > Pausing, off by default). Sarab's own work never counts, so a heavy wallpaper cannot pause itself, and short spikes do not flip it.
- AVI, WMV and MPEG videos play: `sarab set <file>` converts them to MP4 with ffmpeg when it is installed, once, and keeps the result in the library.
- Beta update channel (About > Updates): test builds arrive before everyone else. Stable stays the default.
- App wallpapers: a program (.exe) can run behind the icons. The window asks every time before running one, cycling never picks one, and Sarab ends the program, and anything it started, when the wallpaper changes or Sarab quits.
- `sarab` works in any terminal opened after installing: the installer adds Sarab's folder to your PATH, and uninstalling takes it out. A mistyped command now prints its error in the terminal.
- Right-click a wallpaper card for a menu with its actions: Set, Info, Edit and Delete.

### Changed

- Update notices are easier to act on. The Windows notification stays on screen until answered and has Update now and Later buttons; clicking it opens Sarab. The tray menu gains an Update to X item, and the window asks in its own dialog with the release notes. Sarab checks every 3 hours and whenever the window opens, instead of once a day.
- Wallpapers rest while the Windows screensaver runs, as they do on the lock screen.
- Library cards follow Windows 11: one row of actions that never wraps, with Info, Edit and Delete as icon buttons, equal heights, and a quiet hover. The hover preview fades in instead of flashing black.
- The window no longer shows the browser's right-click menu (Back, Refresh, Print) or reacts to browser shortcuts such as F5 and Ctrl+P. Text boxes keep cut, copy and paste.

### Fixed

- Delete in the library works again; it failed on every wallpaper.

## [0.0.5] - 2026-10-05

### Added

- Change wallpaper every 5, 15 or 30 minutes, or every hour, in library order or at random, through every wallpaper or one category. Next in the tray menu follows the same choice. It waits while the wallpaper rests, and picking a wallpaper yourself restarts the wait.
- The library has a search box, type filters, a category filter and sorting by name, newest or oldest. Search looks at titles, descriptions, tags and categories, in English or Arabic.
- Edit a wallpaper's title, description, author, category (Nature, Space, Abstract, City, Animals, Anime, Games, Vehicles, Minimal, Other) and up to 5 tags.
- Restore defaults in Customize puts a wallpaper's settings back to how it shipped, on that display.
- About > Help: export the log and settings as a zip in Downloads for a bug report, and reset every setting to its default while keeping the library.
- Export a wallpaper from its Info view as a package zip in Downloads. A video or picture added from elsewhere goes inside the zip, so the package works on another PC.
- Move the library to another folder or drive from About > Folders. Every wallpaper moves with it and keeps playing from the new place.
- Library tiles show thumbnails: the Explorer thumbnail for videos, GIFs and pictures, and a frame captured a few seconds after a web page or web address starts playing.
- An Info view for each wallpaper: type, category, tags, author, license, source, size, date added and changed, version, and a button that opens its folder.
- Sound rules in Settings > Performance: mute the wallpaper while another app plays sound (on by default), and play sound only while the desktop has the focus.
- Web wallpapers can ask for system information: with `"api": ["system"]` in sarab.json, the page gets `sarabSystemInfo` once a second while it plays (CPU and GPU name, CPU use, memory, network down and up).
- Web wallpapers can show the song that is playing: with `"api": ["nowplaying"]`, the page gets `sarabNowPlaying` with title, artist, album and cover art from Windows media controls when the track changes.
- Span one wallpaper across every display (Settings > Wallpapers). It rests only when every display would.
- Audio visualizer wallpapers: with `"api": ["audio"]`, the page gets `sarabAudio` with 128 levels from 0 to 1, about 30 times a second, from whatever the PC plays. Capture runs only while such a wallpaper plays.
- Screensaver (Settings > Screensaver): after 1 to 30 idle minutes a wallpaper covers every display, its own or one you pick; any key or mouse move ends it. It waits while a full-screen app is open or another app plays sound, and the desktop wallpapers rest meanwhile.
- Interactive web wallpapers can follow the mouse (Settings > Wallpapers, off by default): moves and left clicks over the desktop reach the wallpaper under the cursor.
- Eight more languages: German, Spanish, French, Portuguese (Brazil), Turkish, Russian, Simplified Chinese and Japanese.
- Video and GIF fit in Settings: fill the screen (cover, the default), fit inside, stretch, or original size.

### Changed

- A wallpaper hidden by an app, a lock or a remote session for a set time (Settings > Performance) is unloaded to free its memory and loads again when it can be seen. On by default, after 5 minutes, on PCs with less than 8 GB of memory.
- Deleting a wallpaper moves its folder to the Recycle Bin, so it can be restored.
- Displays playing the same video now stay within a few milliseconds of each other, under a tenth of a frame, also right after resting: Sarab checks every second and nudges the speed instead of only jumping. YouTube wallpapers on several displays are kept in step too, to about a quarter of a second.
- A display now rests when windows together cover it, such as two apps snapped side by side, and not only when one window fills it.

### Fixed

- YouTube wallpapers play again instead of showing "Video player configuration error" (Error 153). Shorts, live, mobile, YouTube Music and playlist links work too, and pausing and volume reach the YouTube player.

## [0.0.4] - 2026-10-03

### Fixed

- Deleting a wallpaper and reading release notes now open a dialog in Sarab's own style, in light or dark mode and in English or Arabic, instead of a plain browser box titled "tauri.localhost says". Delete starts on **Cancel**, so pressing Enter never removes a wallpaper by accident.

## [0.0.3] - 2026-10-02

First public release, for Windows 10 (1903 or later) and Windows 11.

### Added

- Video, GIF, web page, web address and picture wallpapers, placed under the desktop icons on every display.
- Automatic pausing when an app covers a display, on battery, in energy saver, when the screen is locked, during remote desktop, or for listed apps. Paused wallpapers freeze on their last frame and the window says why.
- The same video on several displays plays in step.
- A window in Windows 11 style: light and dark themes, Acrylic, Mica or solid backdrop, English and Arabic.
- Displays drawn to scale, with a per-display reason and a **Play anyway** button.
- Wallpaper settings from `properties.json`, saved per display.
- The Sarab package format (`sarab.json`) and `.zip` import.
- A command line: `set`, `close`, `pause`, `play`, `resume`, `toggle`, `prop`, `volume`, `next`, `import`, `ui`, `status`, `quit`.
- An About page with the version, license, folders and links.
- A per-user Windows installer in English and Arabic.
- Update notifications: Sarab checks for a new release once a day (Settings can turn this off), shows a Windows notification and a banner, and installs only when you choose **Update now**. Updates are signed and verified before they run.
- 4K presets: public-domain NASA videos (Spinning Earth, Earth from the space station), downloaded only when you choose **Get** and checked against a pinned SHA-256.
- `sarab.json` can set `clip` to play part of a video in a loop.
- Command line: `preset <id>`, `check-update`, `install-update`.
- Opening Sarab from the desktop or Start menu opens its window, also when it already runs in the background. Starting at sign-in stays in the tray.
- Start with Windows, on by default: switched on once at the first launch, and it stays off if you turn it off. Uninstalling removes it.

[Unreleased]: https://github.com/MKAbuMattar/sarab/compare/v0.0.10...HEAD
[0.0.10]: https://github.com/MKAbuMattar/sarab/compare/v0.0.9...v0.0.10
[0.0.9]: https://github.com/MKAbuMattar/sarab/compare/v0.0.8...v0.0.9
[0.0.8]: https://github.com/MKAbuMattar/sarab/compare/v0.0.7...v0.0.8
[0.0.7]: https://github.com/MKAbuMattar/sarab/compare/v0.0.6...v0.0.7
[0.0.6]: https://github.com/MKAbuMattar/sarab/compare/v0.0.5...v0.0.6
[0.0.5]: https://github.com/MKAbuMattar/sarab/compare/v0.0.4...v0.0.5
[0.0.4]: https://github.com/MKAbuMattar/sarab/compare/v0.0.3...v0.0.4
[0.0.3]: https://github.com/MKAbuMattar/sarab/releases/tag/v0.0.3
