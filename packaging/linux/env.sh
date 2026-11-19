#!/usr/bin/env bash
set -euo pipefail
# Wayland / Fractional DPI auto-configuration for Linux WebKitGTK runtimes
if [ -n "${WAYLAND_DISPLAY:-}" ] && [ -z "${GDK_BACKEND:-}" ]; then
  export GDK_BACKEND="wayland,x11"
fi
export WEBKIT_FORCE_COMPOSITING_MODE="${WEBKIT_FORCE_COMPOSITING_MODE:-1}"
