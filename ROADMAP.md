# Roadmap

What comes next for Sarab, in order. Plans change; the [issue tracker](https://github.com/MKAbuMattar/sarab/issues) has the current state, the [changelog](CHANGELOG.md) has what shipped, and [docs/PLAN.md](docs/PLAN.md) has the detailed engineering plan.

The order comes from a comparison with Lively, Sucrose and Wallpaper Engine in October 2026. Linux is the one platform where no free, polished wallpaper app exists, so Linux has to keep Sarab's promise of costing almost nothing when paused before anything else is added there.

## Done

Windows 10 (1903 or later) and 11, on x64, ARM64 and x86:

- Video, GIF, web page, web address, YouTube, picture and app wallpapers under the desktop icons, on every display, or one wallpaper stretched across them all.
- Automatic resting (covered display, battery, energy saver, lock, screensaver, remote desktop, busy CPU, app rules) that freezes in place and says why.
- Video kept in step across displays.
- A library with categories, tags, search and thumbnails; Wallpaper Engine import; Sarab packages to export and share.
- An editor for each wallpaper: details, the part of a video that plays, and the thumbnail (automatic, your own image, or a frame from the video).
- Changing wallpaper every few minutes, in order or at random; a screensaver of your own.
- Web wallpapers can read sound levels, system information and the song that is playing, and follow the mouse. Wallpaper Engine's sound listener and user properties work too.
- A Windows 11 style window in light and dark, Acrylic or Mica, with Sarab's own menus, in ten languages, Arabic right to left.
- A command line (`sarab set`, `pause`, `next`, `status` and more), a per-user installer, and signed, consent-only updates.

Linux, on x64 and ARM64:

- Wallpapers on X11, and on Wayland desktops with layer-shell (KDE Plasma, Sway, Hyprland, COSMIC and others).
- `.deb`, `.rpm`, AppImage and an Arch package in every release.

## Next: 0.1, Linux keeps its promise

Nothing rests on Linux yet, so a laptop runs its wallpaper on battery. This release fixes that first.

- Pausing on battery, lock and idle through logind and UPower. zbus is already in the build, so this adds no dependency.
- Pausing when a window covers the display, on X11. Wayland compositors each report this differently, so it waits until there is a way that works on more than one.
- GNOME on Wayland tested on a real desktop, within a fixed time limit. Where it falls short, the gaps are written down instead of chased.
- A CPU measurement on one Linux machine, on battery, locked and idle, before and after.

Windows, in the same release:

- A file picker for adding wallpapers, next to typing, pasting and dragging.
- Remember **Pause all** across restarts (see [accessibility](ACCESSIBILITY.md)).
- Apply to [SignPath Foundation](https://signpath.org) for free code signing of open-source projects, and to the Microsoft Store, which signs what it lists. Either one stops the SmartScreen warning without a paid certificate.

## Then: 0.2, bring your Lively wallpapers

Lively has a large free catalog that only runs on Windows. Sarab can open it on both.

- First, a test: the 50 most used Lively and Wallpaper Engine web wallpapers on Linux, counting how many look and sound right. Linux renders with WebKitGTK, not Chromium, so WebGL, Web Audio and H.264 video can behave differently. The result decides what this release promises.
- Import Lively wallpapers (`LivelyInfo.json` and `LivelyProperties.json`).
- Lively's web wallpaper API (`livelyAudioListener`, `livelyCurrentTrack`, `livelySystemInformation`, `livelyPropertyListener`), mapped onto what Sarab already sends. Without it, imported wallpapers that react to sound stay silent.
- Sound levels and the song that is playing on Linux, which both the Lively and Wallpaper Engine APIs need.
- More controls in **Customize**: text box, file, and folder, which Lively, Sucrose and Wallpaper Engine wallpapers use.

## Then: 0.3, easier to find

- Flathub, if its sandbox allows a desktop-level window and logind. If it does not, the reason is written down here.
- The Arch package on the AUR.
- `winget` (submitted, waiting for Microsoft's review).
- Then a look at downloads by system before choosing what follows: more Linux work, or playlists.

## Later

In rough order, after 0.3:

- One wallpaper stretched across displays on Wayland (it shows on the first display only for now).
- Linux frame thumbnails, picture wallpapers set as the system wallpaper, the trash for deleting, notifications, and app wallpapers.
- Playlists by time of day, from a folder or tag, and a different playlist on each display.
- A still image of the current wallpaper on the lock screen. Neither Windows nor Lively nor Wallpaper Engine can show a moving one there.
- Pausing when the graphics card, memory or network is busy, or inside a virtual machine.
- Export and import of Sarab's settings.
- Wallpaper Engine's media API (song title, artist and cover art), when someone asks for it.
- Screen reader testing with NVDA and JAWS, and layout checks at 200% text size.

## macOS

Needs a Mac to build, sign and notarize.

- A desktop-level window on every Space, under the Finder icons.
- Pausing from window occlusion, battery, low power mode and lock.
- A signed, notarized `.dmg` for Intel and Apple Silicon.

## Not planned

These exist in other wallpaper apps. Sarab leaves them out on purpose.

- **An online store or gallery.** It needs hosting, moderation, abuse reports and copyright takedowns, which one maintainer cannot carry. Sarab packages can be shared as files, and Lively import (0.2) opens an existing catalog.
- **A scene editor** like Wallpaper Engine's. It is a second product. Web wallpapers cover animation, and the Customize panel covers settings.
- **Unity, Godot and emulator wallpapers.** App wallpapers cover them on Windows.
- **A choice of video players** (mpv, VLC). One built-in player keeps the download small and the idle cost low.
- **Taskbar colouring.** Lively does it through TranslucentTB, a separate app made for it.
- **Discord status, telemetry, ads and a premium tier.** Sarab collects no data about you and stays free.
- **Switching wallpaper when an app starts.** App rules already pause or play for named apps. Ask in an issue if you need more.

## Ideas

Not scheduled. Open an issue to discuss any of them.

- More languages.
