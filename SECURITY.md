# Security policy

## Supported versions

Sarab is in early development. Only the latest release gets security fixes.

| Version | Supported |
|---|---|
| Latest 0.0.x release | Yes |
| Anything older | No, please update |

## Reporting a vulnerability

Please do not open a public issue for a security problem. Use GitHub's private reporting instead: [Report a vulnerability](https://github.com/MKAbuMattar/sarab/security/advisories/new). Only you and the maintainer can see the report.

Include the Sarab version (shown under **About**), the Windows version, what an attacker can do, and the steps or files that show it. A wallpaper package that triggers the problem is the most useful thing you can send.

The maintainer aims to acknowledge a report within 7 days and to agree on a disclosure date with you. Credit goes to you in the release notes unless you ask otherwise.

## What Sarab treats as a security boundary

Wallpapers are untrusted content, so these are in scope:

- A web wallpaper reaching any Sarab command, or any file outside its own package folder.
- A wallpaper page navigating Sarab's window to another origin, or opening new windows.
- A `.zip` package writing outside its target folder (zip slip), or bypassing the size limit.
- Anything that makes Sarab run code or open files the user did not choose.
- The command line or the window opening a path or URL that is not one of Sarab's fixed targets.
- The updater installing anything not signed with Sarab's update key, or a preset download being kept when its size or SHA-256 differs from the pinned value.

Out of scope:

- A web wallpaper doing what any web page can do inside its own sandbox, such as heavy CPU use or loading remote content. Pause or remove it.
- Problems that need an attacker who already controls the user's account or files.
- Missing code signing on the installer. It is a known gap on the [roadmap](ROADMAP.md).
