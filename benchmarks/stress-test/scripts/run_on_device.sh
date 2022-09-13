#!/usr/bin/env bash
set -eux

adb shell am force-stop rust.hotham_stress_test

scriptdir=$(dirname -- "$(realpath -- "$0")")
cd $scriptdir/..

cargo apk run --release

# Wait for the app to start
for i in 1 2 3 4 5; do
    adb shell pidof rust.hotham_stress_test && break
    sleep 1
done

adb logcat --pid="$(adb shell pidof rust.hotham_stress_test)"
