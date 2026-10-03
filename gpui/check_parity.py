"""Gate check: every string key the Tauri window shows is also shown by the GPUI window.

Keys come from ui/index.html (data-t, data-tp, data-tl) and ui/app.js (t('key') and the
t(`prefix.${...}`) families). A key counts as used in GPUI when its quoted name appears in
gpui/src, or for a family, when "prefix.{" does.
"""
import pathlib, re, sys

root = pathlib.Path(__file__).parent
web = root.parent / "ui"
html = (web / "index.html").read_text(encoding="utf-8")
js = (web / "app.js").read_text(encoding="utf-8")
rust = "\n".join(p.read_text(encoding="utf-8") for p in (root / "src").rglob("*.rs"))

keys = set(re.findall(r'data-t[pl]?="([\w.]+)"', html))
keys |= set(re.findall(r"\bt\('([\w.]+)'", js))
families = set(re.findall(r"\bt\(`(\w+)\.\$\{", js))

import json
lang = {l: json.loads((root / "assets" / "i18n" / f"{l}.json").read_text(encoding="utf-8")) for l in ("en", "ar")}
used = {k for k in lang["en"] if f'"{k}"' in rust}


def covered(k):
    # One GPUI label can stand for two Tauri keys with the same text in every language
    # (the Settings component shows one title in the sidebar and the page header).
    same = lambda u: all(lang[l].get(u) == lang[l].get(k) for l in lang)
    return k in used or any(same(u) for u in used)


missing = sorted(k for k in keys if not covered(k))
# A family is covered by a format string, or by naming every key it has in en.json.
en = (root / "assets" / "i18n" / "en.json").read_text(encoding="utf-8")
for f in sorted(families):
    members = re.findall(rf'"({f}\.[\w.]+)"', en)
    if f'"{f}.{{' not in rust and not all(f'"{k}"' in rust for k in members):
        missing.append(f"{f}.*")
if missing:
    print("missing in GPUI:", ", ".join(missing))
    sys.exit(1)
print(f"parity ok: {len(keys)} keys, {len(families)} families")
