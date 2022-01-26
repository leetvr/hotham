#!/usr/bin/env bash
set -eux

adb shell am force-stop rust.beat_saber_example

cd examples/beat-saber-clone
cargo apk run --release

adb logcat --pid="$(adb shell pidof rust.beat_saber_example)"
