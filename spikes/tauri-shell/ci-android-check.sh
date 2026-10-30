#!/usr/bin/env bash
# v0.1 Task 10 CI check: install the debug APK on the running emulator,
# launch the app, and assert `uiautomator dump` contains `RR = 5.57`
# AND the sqlite3mc read-back line (proves the keyed epiora.db open).
# Called as a single line from android-emulator-runner's `script:` input
# (that input runs each line as its own `sh -c`, so the logic lives here).
set -u

APK="${APK:-/home/runner/work/Epiora/Epiora/spikes/tauri-shell/src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk}"

echo "APK=$APK"; ls -l "$APK" || exit 1
adb -s emulator-5554 install -r "$APK" || exit 1
adb -s emulator-5554 shell monkey -p com.epiora.spike -c android.intent.category.LAUNCHER 1

ok=0
for i in $(seq 1 30); do
  adb -s emulator-5554 shell uiautomator dump /sdcard/ui.xml >/dev/null 2>&1 || true
  adb -s emulator-5554 pull /sdcard/ui.xml ui.xml >/dev/null 2>&1 || true
  if grep -q 'RR = 5.57' ui.xml 2>/dev/null \
    && grep -q 'epiora-sqlite3mc-ok' ui.xml \
    && grep -q 'SQLite3 Multiple Ciphers' ui.xml \
    && grep -q 'chacha20' ui.xml; then
    ok=1; break
  fi
  sleep 2
done

echo "uiautomator texts:"
grep -o 'text="[^"]*"' ui.xml 2>/dev/null | grep -v 'text=""' || true

if [ "$ok" != 1 ]; then
  echo "RR = 5.57 and/or sqlite3mc read-back not found in UI"
  head -50 ui.xml 2>/dev/null || true
  exit 1
fi
echo "FOUND: RR = 5.57"
echo "FOUND: sqlite3mc read-back (SQLite3 Multiple Ciphers, chacha20)"
