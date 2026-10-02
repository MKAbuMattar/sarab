# Contributing to Sarab

Thanks for helping. This page covers how to set up, what a change needs before review, and the few rules that keep Sarab light.

By taking part you agree to the [code of conduct](CODE_OF_CONDUCT.md). Security problems go to the [security policy](SECURITY.md), not to public issues.

## Ways to help

- **Report a bug** with the bug report form. Attach `sarab.log` from **About > Settings and logs > Open**.
- **Report an accessibility barrier** with the accessibility form. These are bugs, and they get the same priority.
- **Suggest a feature** with the feature request form. Say what you are trying to do, not only the feature.
- **Translate.** Strings live in `ui/i18n/<lang>.json`. Copy `en.json`, translate the values, keep every key.
- **Write code** from an issue labeled `good first issue` or `help wanted`, or one you opened and discussed first.

## Set up

1. Install Rust stable, the Visual Studio C++ build tools, and the Tauri CLI (`cargo binstall tauri-cli`, or `cargo install --locked tauri-cli --version "^2"`).
2. Clone and build:

   ```powershell
   git clone https://github.com/MKAbuMattar/sarab
   cd sarab
   cargo build --release --manifest-path src-tauri/Cargo.toml
   src-tauri\target\release\sarab.exe
   ```

3. In VS Code, accept the recommended extensions. **Terminal > Run Task** has build, test, installer and check tasks.

The code is in `src-tauri/src` (Rust), `ui/` (the window, plain HTML, CSS and JavaScript with no framework), and `scripts/check.ps1` (the checks). [docs/SYSTEM_DESIGN.md](docs/SYSTEM_DESIGN.md) explains how the pieces fit.

## Before you open a pull request

Run these. CI runs the first four on every pull request.

```powershell
cd src-tauri
cargo fmt --check
cargo clippy --release -- -D warnings
cargo test
cd ..
pwsh -NoProfile -File scripts/check.ps1 static
```

If your change touches embedding, pausing, media or the window, also run the desktop checks that cover it (`embed`, `pause`, `media`, `ui` and so on, listed in the README). CI cannot run them because they need a real desktop. Say in the pull request which ones you ran and what they printed.

If you change what the user sees, attach a screenshot in light and dark mode. If you add a string, add it to every file in `ui/i18n/`; the `static` check fails otherwise.

## Rules that keep Sarab light

1. **Measure performance claims.** A change that may affect CPU or memory comes with `check.ps1 budget` numbers before and after. A paused wallpaper must stay under 0.02 CPU cores.
2. **No new dependency without a reason.** Add a line to the dependency table in [docs/SYSTEM_DESIGN.md](docs/SYSTEM_DESIGN.md) saying why the standard library or an existing crate cannot do it. Check its license: it must be compatible with GPL-3.0.
3. **Platform code stays in `src-tauri/src/os/`.** Everything else should compile unchanged on every platform.
4. **Wallpaper pages get no new powers.** Web wallpapers are untrusted. Data flows from Sarab to the page, never the other way.
5. **Keep the package format stable.** Add fields to `sarab.json` and `properties.json`; never rename or remove one.

## Commits and pull requests

Commit subjects follow `type(scope): subject`, imperative and lowercase, for example `fix(pause): keep the wallpaper visible when an app covers it`. Types: `feat`, `fix`, `refactor`, `perf`, `docs`, `test`, `build`, `ci`, `chore`. Explain the why in the body when the diff does not.

A pull request answers four questions: what changed, why (including any approach you rejected), how a reviewer can verify it, and what is out of scope. The template asks for them.

## License of contributions

Sarab is licensed under the [GNU GPL v3.0](LICENSE). By submitting a contribution you agree that it is licensed under the same terms.
