#!/usr/bin/env bash
set -eux

adb shell am force-stop rust.multiple_guis

scriptdir=$(dirname -- "$(realpath -- "$0")")
cd $scriptdir/..

cargo apk run --release

adb logcat --pid="$(adb shell pidof rust.multiple_guis)"
