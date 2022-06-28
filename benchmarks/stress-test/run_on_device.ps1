${env:RUST_BACKTRACE} = 1
adb shell am force-stop rust.hotham_stress_test

# cargo apk run --release
cargo apk run

if ($?) {
    Start-Sleep -Seconds 2
    $processIdStr = (adb shell pidof rust.hotham_stress_test) | Out-String
    Write-Output $processIdStr
    $processId = $processIdStr -as [int]
    Write-Output $processId
    adb logcat --pid=$processId
}