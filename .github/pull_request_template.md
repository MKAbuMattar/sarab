## What changed

<!-- Two or three plain sentences. Link the issue: Fixes #123 -->

## Why

<!-- The reason, and any approach you tried or rejected, so nobody tries it again. -->

## How to verify

<!-- Commands a reviewer can run and what they print. For visual changes, a screenshot in light and dark mode. -->

```powershell
cd src-tauri; cargo fmt --check; cargo clippy --release -- -D warnings; cargo test; cd ..
pwsh -NoProfile -File scripts/check.ps1 static
```

Desktop checks I ran (and their output):

## Out of scope

<!-- What this pull request leaves for later, if anything visibly unfinished. -->
