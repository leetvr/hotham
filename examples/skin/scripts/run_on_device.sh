#!/usr/bin/env bash
set -eux

adb shell am force-stop rust.skin_example

scriptdir=$(dirname -- "$(realpath -- "$0")")
cd $scriptdir/..

cargo apk run --release

adb logcat --pid="$(adb shell pidof rust.skin_example)"
