#!/usr/bin/env bash
set -eux

adb shell am force-stop rust.complex_scene_example

scriptdir=$(dirname -- "$(realpath -- "$0")")
cd $scriptdir/..

cargo apk build --release --lib
adb install ../../target/release/apk/complex-scene-example.apk
adb shell am start rust.complex_scene_example/android.app.NativeActivity

# Wait for the app to start
for i in 1 2 3 4 5; do
    adb logcat --pid="$(adb shell pidof rust.complex_scene_example)"
    sleep 1
done
