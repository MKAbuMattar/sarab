# Changelog

All notable changes to Sarab are listed here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

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

[Unreleased]: https://github.com/MKAbuMattar/sarab/compare/v0.0.3...HEAD
[0.0.3]: https://github.com/MKAbuMattar/sarab/releases/tag/v0.0.3
