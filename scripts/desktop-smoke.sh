#!/usr/bin/env bash
# Desktop smoke test runner (v0.3 plan Task 15, spec §10.1). Drives the
# built Epiora app through tauri-driver + WebKitWebDriver on Linux:
# installs the driver and session packages, starts a dbus session with
# an unlocked gnome-keyring under Xvfb, then runs WebdriverIO.
#
# Expects the test-hooks release binary at
#   app/src-tauri/target/release/epiora-app   (or $EPIORA_APP_BINARY)
# built from a `VITE_TEST_HOOKS=1 npm run build` dist.
set -euo pipefail
root="$(git rev-parse --show-toplevel)"
cd "$root"

if [ "$(uname -s)" != Linux ]; then
  echo "desktop-smoke.sh is Linux-only" >&2
  exit 2
fi

# 1. System packages (idempotent — apt only runs when something is
#    missing). dbus-run-session ships in dbus; gnome-keyring provides
#    the org.freedesktop.secrets service the keyring crate talks to.
pkgs=()
command -v WebKitWebDriver >/dev/null 2>&1 || pkgs+=(webkit2gtk-driver)
command -v xvfb-run >/dev/null 2>&1 || pkgs+=(xvfb)
command -v gnome-keyring-daemon >/dev/null 2>&1 || pkgs+=(gnome-keyring)
command -v dbus-run-session >/dev/null 2>&1 || pkgs+=(dbus)
if ((${#pkgs[@]})); then
  sudo apt-get update
  sudo apt-get install -y --no-install-recommends "${pkgs[@]}"
fi

# 2. tauri-driver — the plan's `cargo install tauri-driver --locked`;
#    the resolved version is logged for the record.
if ! command -v tauri-driver >/dev/null 2>&1; then
  cargo install tauri-driver --locked
fi
cargo install --list | grep '^tauri-driver' || true

# 3. The binary under test (the hooks build the workflow produces).
bin="${EPIORA_APP_BINARY:-$root/app/src-tauri/target/release/epiora-app}"
[ -x "$bin" ] || {
  echo "no built app at $bin — run 'npx tauri build --no-bundle' first" >&2
  exit 1
}

# 4. A fresh, throwaway home for the app database and the keyring so the
#    first launch exercises key creation, and the spec knows where the
#    database file lands. Exported for the nested session below and the
#    WebdriverIO worker.
work="$(mktemp -d /tmp/epiora-smoke.XXXXXX)"
export XDG_DATA_HOME="$work/data"
export XDG_CONFIG_HOME="$work/config"
export EPIORA_APP_BINARY="$bin"
export EPIORA_APP_DATA_DIR="$XDG_DATA_HOME/org.epiora.app"
export EPIORA_FIXTURE="$root/fixtures/import/utf8_comma/input.csv"

# 5. Re-exec inside a dbus session and an X server: the app binary the
#    driver spawns needs DISPLAY, and secret-service needs a session bus.
if [ "${EPIORA_SMOKE_SESSION:-0}" != 1 ]; then
  export EPIORA_SMOKE_SESSION=1
  exec dbus-run-session -- xvfb-run -a "$0"
fi

# 6. An unlocked keyring (keyring-rs CI recipe): an empty password on
#    --unlock creates login.keyring with no passphrase, then the secrets
#    component serves org.freedesktop.secrets on the session bus.
mkdir -p "$XDG_DATA_HOME/keyrings"
eval "$(echo | gnome-keyring-daemon --unlock)"
eval "$(gnome-keyring-daemon --start --components=secrets)"

# 7. tauri-driver forwards WebDriver to the native driver found on PATH.
tauri-driver --port 4444 &
driver_pid=$!
trap 'kill "$driver_pid" 2>/dev/null || true' EXIT
for _ in $(seq 1 60); do
  curl -fsS http://127.0.0.1:4444/status >/dev/null 2>&1 && break
  sleep 1
done
curl -fsS http://127.0.0.1:4444/status >/dev/null

cd "$root/app"
npx wdio run desktop-e2e/wdio.conf.mjs
