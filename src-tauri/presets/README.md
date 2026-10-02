# Bundled presets

Each folder here is a Sarab package (`sarab.json` at its root) that ships inside the installer and shows in the library as a read-only preset.

Rules for adding one:

- Only code you may redistribute under a GPL-3.0 compatible license. Keep the original license and copyright notice in the package (`LICENSE`), and credit the author in `sarab.json`.
- No outside images, textures, fonts or audio unless their license is checked and included.
- No ports of shaders whose original license is unknown or non-commercial (for example Shadertoy's default CC BY-NC-SA).
- Libraries are vendored inside the package; presets must work offline.
- Animation must run through `requestAnimationFrame`, so Sarab can cap the frame rate and freeze it.
