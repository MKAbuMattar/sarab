#!/usr/bin/env bash
# Live check on Wayland: under a headless Sway, a web wallpaper becomes a layer-shell surface
# (not a tiled window) and its colour fills the screen. Needs: sway, grim, dbus.
# Usage: scripts/check-wayland.sh path/to/sarab
set -euo pipefail
exe=$(realpath "$1")
sb=$(mktemp -d)
export HOME="$sb/home" XDG_CONFIG_HOME="$sb/config" XDG_DATA_HOME="$sb/data"
export XDG_RUNTIME_DIR="$sb/run" WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 WLR_RENDERER=pixman
export WEBKIT_DISABLE_DMABUF_RENDERER=1 WEBKIT_DISABLE_COMPOSITING_MODE=1 GDK_BACKEND=wayland
unset DISPLAY WAYLAND_DISPLAY
mkdir -p "$HOME" "$sb/web" "$XDG_RUNTIME_DIR" && chmod 700 "$XDG_RUNTIME_DIR"
cat > "$sb/web/sarab.json" <<'EOF'
{"title":"Check page","type":"web","file":"index.html","version":1}
EOF
echo '<body style="background:#2F5D6B"></body>' > "$sb/web/index.html"
echo 'output HEADLESS-1 resolution 1920x1080' > "$sb/sway.conf"

sway -c "$sb/sway.conf" > "$sb/sway.log" 2>&1 &
for i in $(seq 20); do
  sock=$(ls "$XDG_RUNTIME_DIR" | grep -E '^wayland-[0-9]+$' | head -1 || true)
  [ -n "$sock" ] && break
  sleep 0.5
done
[ -n "$sock" ] || { echo "FAIL: sway did not start"; cat "$sb/sway.log"; exit 1; }
export WAYLAND_DISPLAY=$sock SWAYSOCK=$(ls "$XDG_RUNTIME_DIR"/sway-ipc.* | head -1)

dbus-run-session -- bash -c '
  set -euo pipefail
  WAYLAND_DEBUG=client "$0" --autostart 2> "$2/wl-debug.log" & sleep 6
  grim -t ppm "$2/before.ppm"
  "$0" set "$1"
  status="$XDG_CONFIG_HOME/com.mkabumattar.sarab/status.json"
  for i in $(seq 30); do
    grep -q "\"loaded\": true" "$status" 2>/dev/null && break
    sleep 1
  done
  grep -q "\"loaded\": true" "$status" || { echo "FAIL: page did not load"; cat "$status"; exit 1; }
  sleep 3
  if swaymsg -t get_tree | grep -q "sarab-wp-"; then
    echo "FAIL: the wallpaper is an ordinary window, not a layer surface"; exit 1
  fi
  swaymsg -t get_tree | grep -E "\"(name|app_id)\"" || true
  grim -t ppm "$2/shot.ppm"
  echo "layer-shell requests:"
  grep -oE "zwlr_layer_(shell|surface)_v1@[0-9]+\.[a-z_]+" "$2/wl-debug.log" | sort | uniq -c | head -20 || true
  grep -E "zwlr_layer_surface_v1@[0-9]+\.configure" "$2/wl-debug.log" | head -3 || true
  python3 - "$2/before.ppm" <<EOF || true
import re, sys
data = open(sys.argv[1], "rb").read()
head = re.match(rb"P6\s+(\d+)\s+(\d+)\s+\d+\s", data)
w = int(head[1]); px = data[head.end():]; i = (int(head[2]) // 2 * w + w // 2) * 3
print("before the wallpaper, centre pixel: #" + px[i:i+3].hex().upper())
EOF
  python3 - "$2/shot.ppm" <<EOF
import re, sys
data = open(sys.argv[1], "rb").read()
head = re.match(rb"P6\s+(\d+)\s+(\d+)\s+\d+\s", data)
w, h = int(head[1]), int(head[2])
px = data[head.end():]
for x, y in [(w // 2, h // 2), (10, 10), (w - 10, h - 10)]:
    r, g, b = px[(y * w + x) * 3:(y * w + x) * 3 + 3]
    print(f"pixel {x},{y}: #{r:02X}{g:02X}{b:02X}")
    assert (r, g, b) == (0x2F, 0x5D, 0x6B), "FAIL: the wallpaper does not fill the screen"
EOF
  "$0" quit || true
  echo "wayland check passed"
' "$exe" "$sb/web" "$sb" || { echo "--- sway log"; tail -30 "$sb/sway.log"; exit 1; }
