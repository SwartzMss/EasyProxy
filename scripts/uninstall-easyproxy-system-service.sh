#!/usr/bin/env sh
set -eu

# Uninstall the system-wide EasyProxy service.
# Stops and disables the service, removes the unit file, and reloads systemd.

bold() { printf "\033[1m%s\033[0m\n" "$*"; }
note() { printf "[info] %s\n" "$*"; }
warn() { printf "[warn] %s\n" "$*"; }

bold "EasyProxy: uninstall system-wide service"

if ! command -v sudo >/dev/null 2>&1; then
  warn "sudo not found; attempting to run systemctl without sudo may fail."
fi

note "Stopping service (if running)"
sudo systemctl stop easyproxy.service 2>/dev/null || true

note "Disabling service (if enabled)"
sudo systemctl disable easyproxy.service 2>/dev/null || true

UNIT_FILE="/etc/systemd/system/easyproxy.service"
if [ -f "$UNIT_FILE" ]; then
  note "Removing unit file: $UNIT_FILE"
  sudo rm -f "$UNIT_FILE"
else
  warn "Unit file not found at $UNIT_FILE (already removed?)"
fi

note "Reloading systemd daemon"
sudo systemctl daemon-reload

bold "Uninstall complete. Useful checks:"
cat <<EOT
  sudo systemctl status easyproxy.service || true
  # Journal logs remain available until rotated: sudo journalctl -u easyproxy.service
EOT
