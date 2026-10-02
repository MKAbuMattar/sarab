# Roadmap

What comes next for Sarab, roughly in order. Plans change; the [issue tracker](https://github.com/MKAbuMattar/sarab/issues) has the current state, and [docs/PLAN.md](docs/PLAN.md) has the detailed engineering plan.

## Now: 0.0.x, Windows

Done in 0.0.3:

- Video, GIF, web page, web address and picture wallpapers under the desktop icons, on every display.
- Automatic pausing (covered display, battery, energy saver, lock, remote desktop, app rules) that freezes in place and explains why.
- Video kept in step across displays.
- Windows 11 style window in light and dark, Acrylic or Mica, English and Arabic.
- Sarab package format, command line, per-user installer.

Still to do for Windows:

- A file picker for adding wallpapers, as an alternative to typing or dragging.
- Thumbnails in the library.
- Code-signed installer, so SmartScreen stops warning.
- Update check that tells you when a new release is out.
- Install through `winget`.
- Remember **Pause all** across restarts (see [accessibility](ACCESSIBILITY.md)).
- Screen reader testing with NVDA and JAWS, and layout checks at 200% text size.

## Next: 0.1, Linux

Needs a Linux machine or virtual machine to build and test.

- Wayland desktops that support the layer-shell protocol: KDE Plasma 6, Sway, Hyprland, COSMIC and others.
- GNOME (the Ubuntu default) through a small GNOME Shell extension.
- X11 window managers.
- Pausing from UPower, power profiles, logind and the compositor.
- Packages: `.deb`, `.rpm`, AppImage, and an AUR package for Arch.

## Then: 0.2, macOS

Needs a Mac to build, sign and notarize.

- A desktop-level window on every Space, under the Finder icons.
- Pausing from window occlusion, battery, low power mode and lock.
- A signed, notarized `.dmg` for Intel and Apple Silicon.

## Later: 0.3, wallpaper features

- Audio-reactive web wallpapers (sound levels sent to the page).
- System information and now-playing track for web wallpapers.
- Mouse interaction with web wallpapers.
- `sarab status` that prints the current state to the terminal.

## Ideas

Not scheduled. Open an issue to discuss any of them.

- Playlists: change the wallpaper every few minutes from a folder or tag.
- One wallpaper stretched across all displays.
- Screensaver mode.
- Native app and game wallpapers on Windows.
- Exporting wallpapers as Sarab packages, and a place to share them.
- More languages.
