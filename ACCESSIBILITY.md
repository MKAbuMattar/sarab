# Accessibility

Sarab should be usable by everyone who wants a living desktop, including people who use a keyboard, a screen reader, a contrast theme, larger text, or reduced motion. An accessibility barrier is a bug, and it gets the same priority as any other bug that blocks someone.

This statement describes Sarab 0.0.3. It was last reviewed on 2 October 2026.

## Goal

The Sarab window aims to meet [WCAG 2.2](https://www.w3.org/TR/WCAG22/) level AA. It has not been audited by a third party.

## Supported environments

| Area | Supported |
|---|---|
| Operating system | Windows 10 (1903 or later) and Windows 11 |
| Window engine | Microsoft WebView2 (Chromium) |
| Screen reader | Windows Narrator, NVDA and JAWS are the targets; none has been tested yet |
| Input | Keyboard, mouse, touch |
| Display settings | Light and dark mode, Windows contrast themes, display scaling, text size, reduced motion (Windows animation effects) |
| Languages | English, Arabic (full right-to-left layout) |

## What Sarab does

- **Contrast:** the brand colors used for text, buttons and badges were measured against WCAG in both themes, and each one used for text is at least 4.5:1. The ratios are listed in [docs/SPEC.md](docs/SPEC.md), section 9. Neutral text and surfaces use the Windows 11 default colors.
- **Not color alone:** each display shows its state as text ("Playing", "Paused", "Resting") and explains why in a sentence, not only with a colored badge.
- **Contrast themes:** under a Windows contrast theme the window switches to system colors and shows selection with outlines.
- **Keyboard:** every control is a native button, input, checkbox or labeled element. Dropdowns open with Enter or Space, move with the arrow keys and close with Escape. Focus is always visible.
- **Reduced motion:** when Windows animation effects are off, the window drops its transitions.
- **Right to left:** in Arabic the whole layout mirrors, while the drawing of your displays keeps their real left and right.
- **Pausing wallpapers:** **Pause all** freezes every wallpaper in one step, from the window or the tray menu. `sarab pause` does the same from the command line or a hotkey tool.

## Known limitations

- **Screen readers:** not tested yet with Narrator, NVDA or JAWS. Controls have labels, but the reading order and announcements of the display map and dropdowns are unverified.
- **Text scaling:** layouts at 200% text size have not been verified.
- **Adding wallpapers** has no file picker yet. You type or paste a path, or drag files onto the window. Drag and drop is not available from the keyboard.
- **Wallpaper content** is made by others. Sarab cannot make a third-party video or web wallpaper accessible, and some may contain flashing images. Pause them with **Pause all**, and lower the frame rate in **Settings**.
- **Pause all** lasts until you choose **Resume**, but it does not survive restarting Sarab or the PC. Remembering it is on the [roadmap](ROADMAP.md).
- **The tray menu** is the standard Windows menu and follows system settings. The tray icon is not reachable with the keyboard except through the notification area itself (Windows key + B).

## Report a barrier

Open an issue with the [accessibility form](https://github.com/MKAbuMattar/sarab/issues/new?template=accessibility.yml). If a barrier stops you from using GitHub itself, ask anyone to open it for you; the form needs no technical detail.

Say what you were trying to do, what you use (for example Narrator, a contrast theme, 150% text), and what got in the way. The maintainer aims to reply within 7 days and to tell you whether and when it can be fixed.
