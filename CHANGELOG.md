# Changelog

All notable changes to Sarab are listed here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Change wallpaper every 5, 15 or 30 minutes, or every hour, in library order or at random, through every wallpaper or one category. Next in the tray menu follows the same choice. It waits while the wallpaper rests, and picking a wallpaper yourself restarts the wait.
- The library has a search box, type filters, a category filter and sorting by name, newest or oldest. Search looks at titles, descriptions, tags and categories, in English or Arabic.
- Edit a wallpaper's title, description, author, category (Nature, Space, Abstract, City, Animals, Anime, Games, Vehicles, Minimal, Other) and up to 5 tags.
- Restore defaults in Customize puts a wallpaper's settings back to how it shipped, on that display.
- About > Help: export the log and settings as a zip in Downloads for a bug report, and reset every setting to its default while keeping the library.
- Export a wallpaper from its Info view as a package zip in Downloads. A video or picture added from elsewhere goes inside the zip, so the package works on another PC.
- An Info view for each wallpaper: type, category, tags, author, license, source, size, date added and changed, version, and a button that opens its folder.
- Sound rules in Settings > Performance: mute the wallpaper while another app plays sound (on by default), and play sound only while the desktop has the focus.
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

[Unreleased]: https://github.com/MKAbuMattar/sarab/compare/v0.0.4...HEAD
[0.0.4]: https://github.com/MKAbuMattar/sarab/compare/v0.0.3...v0.0.4
[0.0.3]: https://github.com/MKAbuMattar/sarab/releases/tag/v0.0.3
