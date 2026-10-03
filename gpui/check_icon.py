"""Gate check: sarab.exe holds assets/icon.ico's images as resources (ID 1 is the window icon)."""
import pathlib, struct, sys

root = pathlib.Path(__file__).parent
ico = (root / "assets" / "icon.ico").read_bytes()
exe = (root / "target" / "release" / "sarab.exe").read_bytes()
count = struct.unpack_from("<H", ico, 4)[0]
missing = 0
for i in range(count):
    size, offset = struct.unpack_from("<II", ico, 6 + 16 * i + 8)
    if ico[offset:offset + min(size, 512)] not in exe:
        missing += 1
print("icon embedded" if missing == 0 else f"{missing} of {count} icon images missing")
sys.exit(1 if missing else 0)
