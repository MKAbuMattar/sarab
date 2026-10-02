# AGENTS.md

Instructions for AI coding agents working in this repository. Humans should start with [README.md](README.md) and [CONTRIBUTING.md](CONTRIBUTING.md); this file adds what an agent gets wrong without being told.

## What this is

Sarab is a Windows animated-wallpaper app: Rust and Tauri v2 on the system WebView2. Wallpaper windows sit under the desktop icons and freeze in place when an app covers the display. Version 0.0.3. Only the Windows backend exists; Linux and macOS are planned ([ROADMAP.md](ROADMAP.md)).

## Layout

| Path | What |
|---|---|
| `src-tauri/src/main.rs` | Tray, single instance, CLI dispatch, Tauri commands for the settings window, the 1 s tick thread |
| `src-tauri/src/wallpaper.rs` | Core state: displays, wallpaper windows, pause states, status file, video sync |
| `src-tauri/src/pause.rs` | `decide()`: signals in, state and reason per display out. Pure, unit tested |
| `src-tauri/src/library.rs` | `sarab.json` packages, zip import, `properties.json` |
| `src-tauri/src/os/windows.rs` | Desktop embedding (Progman, WorkerW), pause probes, OS wallpaper API |
| `src-tauri/src/inject.js` | Injected into every wallpaper page: frame cap, freeze, video sync |
| `ui/` | The settings window. Plain HTML, CSS and JS, no framework, no bundler |
| `scripts/check.ps1` | Every check, one per gate name |
| `docs/` | Specification, system design, plan |

## Commands

Run from the repository root unless noted.

```powershell
cd src-tauri; cargo fmt --check; cargo clippy --release -- -D warnings; cargo test; cd ..
pwsh -NoProfile -File scripts/check.ps1 static
pwsh -NoProfile -File scripts/check.ps1 brand
cd src-tauri; cargo tauri build          # app and installer
```

A check passes only when it exits 0 and prints `<name> gate passed`. Read the output; do not report a pass you did not see.

## Checks and the user's desktop

- **Safe anywhere:** `unit`, `static`, `brand`, `about`.
- **Need the Windows desktop** and run Sarab in a sandboxed `APPDATA` under `scripts/.sandbox`: `embed`, `media`, `pause`, `budget`, `prop`, `restore`, `kill`, `ui`, `switch`, `fullscreen`, `reasons`, `sync`, `installer`.
- **Ask the user first:** `picture` changes and then restores the Windows wallpaper; `explorer` restarts explorer.exe and closes File Explorer windows. Never minimize, close or move the user's windows without asking.
- `pause` refuses to run when every display is covered by a maximized window. That is correct; do not weaken it.
- `budget` has measured one outlier in four runs (0.024 cores against 0.02, the rest 0.002 to 0.006). Rerun before concluding anything.

## Before building or testing

1. **A running `sarab.exe` locks the build output** and, through single instance, swallows the checks' commands. Find it with `Get-Process sarab`. If the user started it (parent `explorer.exe`), ask before quitting it, quit it with `sarab quit`, and start it again when you are done.
2. **The user's real data** is in `%APPDATA%\com.mkabumattar.sarab` and `%LOCALAPPDATA%\com.mkabumattar.sarab`. Never write there from a check; never delete anything there without asking.

## Rules

1. **No new dependency** without a line in the dependency table of `docs/SYSTEM_DESIGN.md`, and its license must be GPL-3.0 compatible. Sarab is GPL-3.0-only. It cannot move to GPL-2.0-only: `tao` is Apache-2.0 only.
2. **Platform code stays in `src-tauri/src/os/`.**
3. **Wallpaper pages are untrusted.** They get no Tauri capability (only the `main` window does, see `capabilities/main.json`), and data flows host to page only, through `call()` in `wallpaper.rs`. Never pass page-controlled text into `eval` except as a `serde_json` literal.
4. **The `open` command takes fixed names only**, never a path or URL from the page.
5. **Keep the package format stable.** Add fields to `sarab.json` and `properties.json`; never rename or remove one.
6. **Every UI string goes in every `ui/i18n/*.json`.** English and Arabic must have the same keys; `static` fails otherwise. Arabic is right to left: use logical CSS properties (`inset-inline-start`, `margin-inline`), never `left` or `right`, except in the display map, which keeps physical order.
7. **No other wallpaper app is named** anywhere in the repository, code, docs or comments. `brand` scans every file.
8. **No email addresses.** The project has no mailbox; contact goes through GitHub (issues, private vulnerability reporting).
9. **GitHub Actions are pinned by full commit SHA** with the version as a comment, for example `actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1`. Resolve a new pin with `gh api repos/<owner>/<repo>/commits/<tag> --jq .sha`, and check the action's inputs at that commit.
10. **Version** lives in `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json` and must match; `CHANGELOG.md` needs a section for it. `static` checks both.

## Traps this code has already hit

| Trap | Why | Where |
|---|---|---|
| Hiding a covered wallpaper window | The plain Windows wallpaper showed through covering apps and during alt-tab. Pausing freezes in place instead | `apply_state` in `wallpaper.rs` |
| Holding the core lock across `on_main` or inside WebView2 callbacks | Deadlock. `run_on_main_thread` runs inline on the main thread. Page-load and eval callbacks go through `later()` | `on_main`, `later` in `wallpaper.rs`; `with_core` in `main.rs` |
| Reusing a window label after `destroy()` | Destroy frees the label asynchronously, so the replacement fails with "label already exists". Labels come from `new_label()` | `wallpaper.rs` |
| Trusting `prefers-color-scheme` | WebView2 keeps the color scheme per profile, shared by every Sarab window. The page gets `data-theme` from `page_theme()` | `main.rs`, `ui/style.css` |
| Native `<select>` popups | WebView2 draws them white in dark mode. `combo()` in `ui/app.js` draws a Fluent list over a hidden select | `ui/app.js` |
| WorkerW above the wallpaper on the raised desktop | Shows the Windows wallpaper. `ensure_order()` repairs and logs it every tick | `os/windows.rs` |
| `SystemParametersInfo(SPI_SETDESKWALLPAPER)` on the raised desktop | Destroys the WorkerW. `refresh_desktop()` skips it there | `os/windows.rs` |
| The per-user install folder is `%LOCALAPPDATA%\Sarab` | Data in that folder could go with an uninstall, so data uses the app identifier folders | `settings.rs` |
| An app identifier ending in `.app` | Clashes with macOS bundles. It is `com.mkabumattar.sarab` | `tauri.conf.json` |

## Writing

- Commits: `type(scope): subject`, imperative, lowercase, 72 characters at most, body for the why and any rejected approach. Never add AI attribution lines (`Co-Authored-By` naming a model, "Generated with").
- Docs and UI text: no em or en dashes, sentence-case headings, plain words. Say what was measured, with the number, and mark what was not tested as untested.
- Commit or push only when the user asks. The repository is not published yet.
