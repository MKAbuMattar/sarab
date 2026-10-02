"""Draw the Sarab mark (a three-petal iris in Wadi Rum Sand on a Black Iris square) as PNG and ICO. Stdlib only."""
import math, struct, zlib, sys, os

IRIS, SAND = (0x2B, 0x22, 0x33), (0xD9, 0xA3, 0x6A)

def inside_petal(x, y, angle, length, width):
    # Ellipse petal rooted at the center, pointing along `angle` (screen coords, y down).
    c, s = math.cos(math.radians(angle)), math.sin(math.radians(angle))
    u, v = x * c + y * s, -x * s + y * c
    return ((u - length) / length) ** 2 + (v / width) ** 2 <= 1

# Iris: a tall upright standard, two narrow side standards, two drooping falls.
PETALS = [(-90, 0.17, 0.075), (-125, 0.13, 0.05), (-55, 0.13, 0.05), (140, 0.15, 0.065), (40, 0.15, 0.065)]

def pixel(x, y, tray):
    # x, y in [-0.5, 0.5]
    r, half = 0.18, 0.5
    qx, qy = max(abs(x) - (half - r), 0), max(abs(y) - (half - r), 0)
    in_square = qx * qx + qy * qy <= r * r
    cy = y + 0.02
    flower = any(inside_petal(x, cy, a, l, w) for a, l, w in PETALS) or (x * x + cy * cy) <= 0.045 ** 2
    stem = abs(x) < 0.022 and 0.02 < y < 0.38
    if tray:
        return (255, 255, 255, 255) if (flower or stem) else (0, 0, 0, 0)
    if not in_square:
        return (0, 0, 0, 0)
    return SAND + (255,) if (flower or stem) else IRIS + (255,)

def render(n, tray=False, ss=4):
    rows = []
    for j in range(n):
        row = bytearray([0])
        for i in range(n):
            acc = [0, 0, 0, 0]
            for a in range(ss):
                for b in range(ss):
                    p = pixel((i + (a + 0.5) / ss) / n - 0.5, (j + (b + 0.5) / ss) / n - 0.5, tray)
                    for k in range(4):
                        acc[k] += p[k] * (p[3] if k < 3 else 255)
            alpha = acc[3] / (ss * ss * 255)
            rgb = [min(255, int(acc[k] * 255 / acc[3])) for k in range(3)] if acc[3] else [0, 0, 0]
            row += bytes(rgb + [int(round(alpha))])
        rows.append(bytes(row))
    def chunk(t, d): return struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d) & 0xffffffff)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", n, n, 8, 6, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(b"".join(rows), 9)) + chunk(b"IEND", b"")

def ico(pngs):
    head = struct.pack("<HHH", 0, 1, len(pngs))
    offset, dirs, data = 6 + 16 * len(pngs), b"", b""
    for n, png in pngs:
        dirs += struct.pack("<BBBBHHII", n % 256, n % 256, 0, 0, 1, 32, len(png), offset + len(data))
        data += png
    return head + dirs + data

out = sys.argv[1] if len(sys.argv) > 1 else "."
for name, n in [("32x32.png", 32), ("128x128.png", 128), ("icon.png", 512)]:
    open(os.path.join(out, name), "wb").write(render(n))
open(os.path.join(out, "icon.ico"), "wb").write(ico([(n, render(n)) for n in (16, 32, 48, 256)]))
open(os.path.join(out, "tray.png"), "wb").write(render(32, tray=True))
print("icons written")
