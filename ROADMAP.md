# Roadmap

What comes next for Sarab, roughly in order. Plans change; the [issue tracker](https://github.com/MKAbuMattar/sarab/issues) has the current state, the [changelog](CHANGELOG.md) has what shipped, and [docs/PLAN.md](docs/PLAN.md) has the detailed engineering plan.

## Done

Windows 10 (1903 or later) and 11, on x64, ARM64 and x86:

- Video, GIF, web page, web address, YouTube, picture and app wallpapers under the desktop icons, on every display, or one wallpaper stretched across them all.
- Automatic resting (covered display, battery, energy saver, lock, screensaver, remote desktop, busy CPU, app rules) that freezes in place and says why.
- Video kept in step across displays.
- A library with categories, tags, search and thumbnails; Wallpaper Engine import; Sarab packages to export and share.
- Changing wallpaper every few minutes, in order or at random; a screensaver of your own.
- Web wallpapers can read sound levels, system information and the song that is playing, and follow the mouse.
- A Windows 11 style window in light and dark, Acrylic or Mica, with Sarab's own menus, in ten languages, Arabic right to left.
- A command line (`sarab set`, `pause`, `next`, `status` and more), a per-user installer, and signed, consent-only updates.

## Now: 0.1, Linux

Built and tested in CI on Ubuntu, Fedora and Arch, for x64 and ARM64. Still to do:

- Pausing from UPower, power profiles, logind and the compositor. Nothing rests on Linux yet.
- Linux packages in each release: `.deb`, `.rpm`, AppImage, and the Arch package on the AUR.
- GNOME on Wayland: Sarab runs there through XWayland, not yet tested on a real GNOME desktop. A small GNOME Shell extension if that is not enough.
- One wallpaper stretched across displays on Wayland (it shows on the first display only for now).
- Picture wallpapers set as the system wallpaper, sound levels and now playing for web wallpapers, the trash for deleting, notifications, and app wallpapers.

Done for Linux: wallpapers on X11, and on Wayland desktops with layer-shell (KDE Plasma, Sway, Hyprland, COSMIC and others).

## Still to do for Windows

- A file picker for adding wallpapers, next to typing, pasting and dragging.
- A code-signed installer, so SmartScreen stops warning.
- Install through `winget` (submitted, waiting for review).
- Remember **Pause all** across restarts (see [accessibility](ACCESSIBILITY.md)).
- Screen reader testing with NVDA and JAWS, and layout checks at 200% text size.

## Then: 0.2, macOS

Needs a Mac to build, sign and notarize.

- A desktop-level window on every Space, under the Finder icons.
- Pausing from window occlusion, battery, low power mode and lock.
- A signed, notarized `.dmg` for Intel and Apple Silicon.

## Ideas

Not scheduled. Open an issue to discuss any of them.

- A place to share Sarab packages.
- Playlists from a folder or tag, beyond one category.
- More languages.
