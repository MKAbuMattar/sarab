# Sarab · سراب

[![CI](https://github.com/MKAbuMattar/sarab/actions/workflows/ci.yml/badge.svg)](https://github.com/MKAbuMattar/sarab/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/MKAbuMattar/sarab?include_prereleases)](https://github.com/MKAbuMattar/sarab/releases)
[![License: GPL v3](https://img.shields.io/badge/license-GPL--3.0-blue)](LICENSE)

Animated desktop wallpapers for Windows that rest when you work.

Sarab plays videos, GIFs, web pages and pictures behind your desktop icons, on one display or many. It freezes the wallpaper whenever an app covers the screen, the laptop runs on battery, or the screen is locked, so it stays light on old PCs. Sarab (سراب) means mirage in Arabic.

Website: [sarab.mkabumattar.com](https://sarab.mkabumattar.com)

## Status

Version 0.0.3, an early release for **Windows 10 (1903 or later) and Windows 11**. Linux and macOS are planned; see the [roadmap](ROADMAP.md).

## What it does

- **Wallpaper types:** video (mp4, webm, mov and more), GIF, web page (a folder with `index.html`), web address, picture.
- **Every display:** one wallpaper on all displays, or a different one on each. The same video on several displays plays in step.
- **Pauses itself:** when an app covers a display, on battery, in energy saver, when locked, during remote desktop, or for apps you list. A paused wallpaper freezes on its last frame, and the window says why it paused.
- **Cheap when paused:** about 0.005 CPU cores on the test machine with two web wallpapers, measured by `scripts/check.ps1 budget`.
- **Looks like Windows 11:** light and dark themes, Acrylic or Mica backdrop, English and Arabic (right to left).
- **Command line:** every action is also a `sarab` command, so scripts and hotkey tools can drive it.

## Install

1. Download `Sarab_<version>_x64-setup.exe` from [Releases](https://github.com/MKAbuMattar/sarab/releases).
2. Run it. It installs for your user only, so it needs no administrator rights, and it asks for English or Arabic.
3. Windows SmartScreen may warn about an unknown publisher, because the installer is not code-signed yet. Choose **More info**, then **Run anyway**.

The installer downloads the Microsoft WebView2 runtime if the PC does not have it (Windows 11 always does).

To remove Sarab, use **Settings > Apps > Installed apps > Sarab > Uninstall**. Your settings and library stay unless you tick "Delete the application data".

## Use

Sarab lives in the notification area as an iris icon. Left-click it to open the window.

- **Library:** your displays are drawn at the top. Select one to change only that display, or leave **All displays** selected. Pick a wallpaper card and choose **Set**.
- **Add wallpapers:** paste a file path, folder or web address and choose **Add**, or drop files on the window.
- **Settings:** theme, transparency, language, pause rules, frame rate, volume, apps that always pause or always play, start with Windows.
- **About:** version, license, and buttons that open the settings, log and library folders.

## Command line

A second `sarab` process hands its arguments to the running one and exits.

| Command                                                      | Does                                                              |
| ------------------------------------------------------------ | ----------------------------------------------------------------- |
| `sarab set <file, folder, URL, or library id> [--display N]` | Apply to one display or all. Display 0 is the leftmost            |
| `sarab close [--display N]`                                  | Close, and put back the Windows wallpaper if Sarab changed it     |
| `sarab pause` / `play` / `resume` / `toggle`                 | Manual pause, play regardless of rules, back to automatic, toggle |
| `sarab prop <key>=<value> [--display N]`                     | Set a value from the wallpaper's `properties.json`                |
| `sarab volume <0-100>`                                       | Wallpaper volume (0 mutes)                                        |
| `sarab next`                                                 | Next wallpaper in the library, on every display                   |
| `sarab import <zip>`                                         | Import a Sarab package                                            |
| `sarab ui` / `status` / `quit`                               | Open the window, write `status.json`, exit                        |

## Wallpaper packages

A package is a folder, or a `.zip` of one, with `sarab.json` at its root:

```json
{
  "title": "Rain",
  "type": "web",
  "file": "index.html",
  "author": "you",
  "tags": ["calm"],
  "version": 1
}
```

`type` is `web`, `url`, `video`, `gif` or `picture`. An optional `properties.json` lists controls the user can change: `slider`, `checkbox`, `dropdown`, `color`, `textbox`, `button`, `label`. A web wallpaper receives them through functions it defines:

```js
window.sarabPropertyChanged = (name, value) => {
  /* on load, and on every change */
};
window.sarabPlaybackChanged = ({ paused }) => {
  /* stop timers while paused */
};
```

Web wallpapers run in an isolated WebView2 window with no access to Sarab's commands. Only the files inside the package are reachable.

## Files

| What                                 | Where                                          |
| ------------------------------------ | ---------------------------------------------- |
| Settings, layout, logs (`sarab.log`) | `%APPDATA%\com.mkabumattar.sarab`              |
| Library                              | `%LOCALAPPDATA%\com.mkabumattar.sarab\Library` |
| The app (installer)                  | `%LOCALAPPDATA%\Sarab`                         |

## Build from source

You need Rust (stable), the Visual Studio C++ build tools, and the Tauri CLI.

```powershell
cargo install --locked tauri-cli --version "^2"   # or: cargo binstall tauri-cli
cargo build --release --manifest-path src-tauri/Cargo.toml   # app only
cd src-tauri; cargo tauri build                              # app and installer
```

The installer lands in `src-tauri/target/release/bundle/nsis/`.

Opening the repository in VS Code suggests the extensions it uses and adds build, test and check tasks (**Terminal > Run Task**).

## Checks

`pwsh -NoProfile -File scripts/check.ps1 <check>` runs one check. Each prints `<check> gate passed` only when every assertion holds.

| Group      | Checks                                                                                                          | Needs                                           |
| ---------- | --------------------------------------------------------------------------------------------------------------- | ----------------------------------------------- |
| Static     | `unit`, `static`, `brand`                                                                                       | Nothing beyond Rust                             |
| Desktop    | `embed`, `media`, `pause`, `budget`, `prop`, `restore`, `kill`, `ui`, `switch`, `fullscreen`, `reasons`, `sync` | A Windows 11 desktop; `sync` needs two displays |
| Release    | `about`, `installer`                                                                                            | A built installer                               |
| Disruptive | `picture` changes and then restores your Windows wallpaper; `explorer` restarts explorer.exe                    | Run only when that is fine                      |

Desktop checks run Sarab with a sandboxed `APPDATA` under `scripts/.sandbox`, so your own settings are never touched.

## Documents

- [Specification](docs/SPEC.md), [system design](docs/SYSTEM_DESIGN.md), [plan](docs/PLAN.md), [roadmap](ROADMAP.md), [changelog](CHANGELOG.md)
- [Contributing](CONTRIBUTING.md), [code of conduct](CODE_OF_CONDUCT.md), [security policy](SECURITY.md), [accessibility](ACCESSIBILITY.md)

## License

Sarab is free software under the [GNU General Public License v3.0](LICENSE). The colors come from the [Jordanian Identity Colors](https://github.com/MKAbuMattar/black-iris/blob/main/.github/assets/jordan-identity-colors.json) palette.
