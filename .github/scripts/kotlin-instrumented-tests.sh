#!/usr/bin/env bash
# Called from packages/kotlin-sdk by the image's ci-android-emulator wrapper.
set -euo pipefail

# Keystore's unlocked-device-required keys fail if the screen re-locks mid-test.
adb shell settings put system screen_off_timeout 2147483647
adb shell svc power stayon true

# Auth-required identity keys need an enrolled secure lockscreen on the test AVD.
adb shell locksettings set-pin 1234

# Cold boot can race credential acceptance and keyguard dismissal. Preserve the
# upstream retry and trust-state gate, now in one shell (not line-by-line sh -c).
unlocked=false
for attempt in 1 2 3; do
  adb shell input keyevent KEYCODE_WAKEUP
  adb shell wm dismiss-keyguard
  adb shell input text 1234
  adb shell input keyevent KEYCODE_ENTER
  sleep 2
  adb shell wm dismiss-keyguard
  sleep 1
  if adb shell dumpsys trust | grep -q 'deviceLocked=0'; then
    unlocked=true
    break
  fi
done
if [ "$unlocked" != true ]; then
  echo '::error::Emulator is still locked; Keystore-backed tests would fail spuriously.'
  adb shell dumpsys trust
  exit 1
fi

./gradlew :sdk:connectedDebugAndroidTest --stacktrace
