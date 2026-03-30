#!/usr/bin/env bash
set -eux

adb shell am force-stop rust.complex_scene_example

scriptdir=$(dirname -- "$(realpath -- "$0")")
cd $scriptdir/..

adb shell am start rust.complex_scene_example/android.app.NativeActivity

adb logcat --pid="$(adb shell pidof rust.complex_scene_example)"
