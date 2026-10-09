#!/usr/bin/env bash
# Live check on X11: a web wallpaper becomes a desktop-type window, full screen, below everything.
# Needs: Xvfb, openbox, xdotool, x11-utils, dbus. Usage: scripts/check-linux.sh path/to/sarab
set -euo pipefail
exe=$(realpath "$1")
sb=$(mktemp -d)
export HOME="$sb/home" XDG_CONFIG_HOME="$sb/config" XDG_DATA_HOME="$sb/data" DISPLAY=:99
mkdir -p "$HOME" "$sb/web"
cat > "$sb/web/sarab.json" <<'EOF'
{"title":"Check page","type":"web","file":"index.html","version":1}
EOF
echo '<body style="background:#2F5D6B"></body>' > "$sb/web/index.html"

Xvfb :99 -screen 0 1920x1080x24 & sleep 2
openbox & sleep 1

dbus-run-session -- bash -c '
  "$0" --autostart & sleep 6
  "$0" set "$1"
  for i in $(seq 30); do
    id=$(xdotool search --name "^sarab-wp-0$" 2>/dev/null | head -1 || true)
    [ -n "$id" ] && break
    sleep 1
  done
  [ -n "$id" ] || { echo "FAIL: no wallpaper window"; exit 1; }
  sleep 3
  echo "window $id"
  type=$(xprop -id "$id" _NET_WM_WINDOW_TYPE)
  echo "$type"
  echo "$type" | grep -q _NET_WM_WINDOW_TYPE_DESKTOP || { echo "FAIL: not a desktop window"; exit 1; }
  geo=$(xwininfo -id "$id" | grep -E "Width|Height" | tr -s " " | cut -d" " -f3 | tr "\n" x)
  echo "size $geo"
  [ "$geo" = "1920x1080x" ] || { echo "FAIL: not full screen"; exit 1; }
  stack=$(xprop -root _NET_CLIENT_LIST_STACKING | sed "s/.*# //" | tr -d " ")
  echo "stacking (bottom first): $stack"
  first=$(printf "0x%x" "$id")
  [ "${stack%%,*}" = "$first" ] || { echo "FAIL: wallpaper is not the bottom window"; exit 1; }
  status="$XDG_CONFIG_HOME/com.mkabumattar.sarab/status.json"
  [ -f "$status" ] || status=$(find "$HOME" "$XDG_CONFIG_HOME" "$XDG_DATA_HOME" -name status.json | head -1)
  echo "status: $status"
  grep -q "\"loaded\": true" "$status" || { echo "FAIL: page did not load"; cat "$status"; exit 1; }
  "$0" quit || true
  echo "x11 check passed"
' "$exe" "$sb/web"
