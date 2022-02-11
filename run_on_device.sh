#!/usr/bin/env bash
set -eux

adb shell am force-stop rust.crab_saber

cd examples/crab-saber
cargo apk run --release

adb logcat --pid="$(adb shell pidof rust.crab_saber)"
