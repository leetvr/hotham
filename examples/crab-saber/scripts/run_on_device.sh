#!/usr/bin/env bash
set -eux

adb shell am force-stop rust.crab_saber

scriptdir=$(dirname -- "$(realpath -- "$0")")
cd $scriptdir/..

cargo apk run --release --package crab-saber

# Wait for the app to start
for i in 1 2 3 4 5; do
    adb shell pidof rust.crab_saber && break
    sleep 1
done

adb logcat --pid="$(adb shell pidof rust.crab_saber)"
