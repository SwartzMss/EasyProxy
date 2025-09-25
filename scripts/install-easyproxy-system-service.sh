#!/usr/bin/env sh
set -eu

# Install a system-wide systemd service for EasyProxy and start it.
# This script writes /etc/systemd/system/easyproxy.service and runs the
# service as the current non-root user to avoid certificate permission issues.

bold() { printf "\033[1m%s\033[0m\n" "$*"; }
note() { printf "[info] %s\n" "$*"; }
warn() { printf "[warn] %s\n" "$*"; }
err()  { printf "[err ] %s\n" "$*"; }

UID_CUR=$(id -u)
if [ "$UID_CUR" -eq 0 ]; then
  warn "It's recommended to run this script as a normal user."
  warn "The installed service will run as a non-root user."
fi

SCRIPT_DIR=$(CDPATH=; cd "$(dirname "$0")" 2>/dev/null && pwd -P)
REPO_DIR=$(CDPATH=; cd "$SCRIPT_DIR/.." 2>/dev/null && pwd -P)

# Detect package/binary name from Cargo.toml [package] name
PKG_NAME=$(awk '
  BEGIN{sec=""}
  /^\[/ {sec=$0; next}
  sec=="[package]" && $1=="name" {
    if (match($0, /"([^"]+)"/, m)) { print m[1]; exit }
  }
' "$REPO_DIR/Cargo.toml" 2>/dev/null || true)

# Fallback if detection failed
if [ -z "$PKG_NAME" ]; then PKG_NAME=easyproxy; fi

# Prefer exact-case binary name, then lowercase
BIN="$REPO_DIR/target/release/$PKG_NAME"
if [ ! -x "$BIN" ]; then
  LC_NAME=$(echo "$PKG_NAME" | tr '[:upper:]' '[:lower:]')
  if [ -x "$REPO_DIR/target/release/$LC_NAME" ]; then
    BIN="$REPO_DIR/target/release/$LC_NAME"
  fi
fi
ENV_FILE="$REPO_DIR/.env"
RUN_USER=${RUN_USER-$(id -un)}
RUN_GROUP=${RUN_GROUP-$(id -gn)}

bold "EasyProxy: install system-wide systemd service"
note "Repo directory: $REPO_DIR"
note "Run user/group: $RUN_USER:$RUN_GROUP"

if [ ! -f "$ENV_FILE" ]; then
  warn ".env not found at $ENV_FILE"
  warn "The service will still start, but your app currently exits when .env is missing."
  warn "Create $ENV_FILE or adjust the code to not exit if dotenv is missing."
fi

if [ ! -x "$BIN" ]; then
  warn "Binary not found at $BIN"
  warn "Building release binary... (cargo build --release)"
  if ! command -v cargo >/dev/null 2>&1; then
    err "cargo not available in PATH. Please build manually: 'cargo build --release'"
    exit 1
  fi
  (cd "$REPO_DIR" && cargo build --release)
  # Re-evaluate BIN after build
  if [ -x "$REPO_DIR/target/release/$PKG_NAME" ]; then
    BIN="$REPO_DIR/target/release/$PKG_NAME"
  elif [ -x "$REPO_DIR/target/release/$LC_NAME" ]; then
    BIN="$REPO_DIR/target/release/$LC_NAME"
  fi
fi

if [ ! -x "$BIN" ]; then
  err "Built binary still not found (looked for '$PKG_NAME' and '$LC_NAME')."
  err "Please verify the binary name under target/release and rerun."
  exit 1
fi

UNIT_CONTENT=$(cat <<EOF
[Unit]
Description=EasyProxy (system service)
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=$RUN_USER
Group=$RUN_GROUP
ExecStart=$BIN
WorkingDirectory=$REPO_DIR
EnvironmentFile=$ENV_FILE
Restart=on-failure
RestartSec=3

[Install]
WantedBy=multi-user.target
EOF
)

bold "Writing /etc/systemd/system/easyproxy.service (sudo required)"
printf "%s" "$UNIT_CONTENT" | sudo tee /etc/systemd/system/easyproxy.service >/dev/null

sudo systemctl daemon-reload
sudo systemctl enable --now easyproxy.service

bold "Service enabled and started. Useful commands:"
cat <<EOT
  sudo systemctl status easyproxy.service
  sudo journalctl -u easyproxy.service -f
  sudo systemctl restart easyproxy.service
  sudo systemctl stop easyproxy.service
EOT

note "If the service fails to read cert/key in your home, ensure permissions allow user $RUN_USER to read them."
