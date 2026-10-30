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

# --- Plan Task 10 Step 4: cold start on the D7 proxy ---
# The D7 reference is the CI emulator (owner's Task 8 decision;
# real-device confirmation deferred to a later plan). The aarch64
# release APK cannot run on this x86_64 emulator, so cold start is
# measured on the x86_64 DEBUG APK installed above.
echo "=== D7 proxy environment (emulator, not a physical device) ==="
echo "api-level: $(adb -s emulator-5554 shell getprop ro.build.version.sdk | tr -d '\r')"
echo "android-release: $(adb -s emulator-5554 shell getprop ro.build.version.release | tr -d '\r')"
echo "image-fingerprint: $(adb -s emulator-5554 shell getprop ro.build.fingerprint | tr -d '\r')"
echo "arch/abi: $(adb -s emulator-5554 shell getprop ro.product.cpu.abilist | tr -d '\r')"
echo "hardware: $(adb -s emulator-5554 shell getprop ro.hardware | tr -d '\r')  ro.kernel.qemu: $(adb -s emulator-5554 shell getprop ro.kernel.qemu | tr -d '\r')"
echo "emulator image: api-level 34 / target google_apis / arch x86_64"
echo "emulator-options: -no-snapshot-save -no-window -gpu swiftshader_indirect -noaudio -no-boot-anim"
echo "host KVM device: $(ls -l /dev/kvm 2>&1)"

# Launcher activity, resolved from the installed package's manifest.
ACTIVITY="$(adb -s emulator-5554 shell cmd package resolve-activity --brief \
  -a android.intent.action.MAIN -c android.intent.category.LAUNCHER \
  com.epiora.spike 2>/dev/null | tr -d '\r' | grep 'com.epiora.spike/' | head -n 1)"
echo "launcher activity (resolved from the installed manifest): ${ACTIVITY:-<none>}"
[ -n "$ACTIVITY" ] || ACTIVITY="com.epiora.spike/.MainActivity"

echo "=== Cold start x3 on the D7 proxy: x86_64 debug APK (the aarch64 release APK cannot run on the x86_64 emulator) ==="
for run in 1 2 3; do
  adb -s emulator-5554 shell am force-stop com.epiora.spike
  sleep 2
  echo "--- cold start run $run: adb shell am start -W -n $ACTIVITY"
  adb -s emulator-5554 shell am start -W -n "$ACTIVITY" | tr -d '\r'
  sleep 2
done
