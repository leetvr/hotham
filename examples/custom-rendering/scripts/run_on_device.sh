#!/usr/bin/env bash
set -eux

adb shell am force-stop rust.custom_rendering_example

scriptdir=$(dirname -- "$(realpath -- "$0")")
cd $scriptdir/..

cargo apk run --release

# Wait for the app to start
for i in 1 2 3 4 5; do
    adb shell pidof rust.custom_rendering_example && break
    sleep 1
done

adb logcat --pid="$(adb shell pidof rust.custom_rendering_example)"
