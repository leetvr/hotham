${env:RUST_BACKTRACE} = 1
adb shell am force-stop rust.hotham_beat_saber_example

cargo apk run --release

if ($?) {
    Start-Sleep -Seconds 2
    $processIdStr = (adb shell pidof rust.beat_saber_example) | Out-String
    Write-Output $processIdStr
    $processId = $processIdStr -as [int]
    Write-Output $processId
    adb logcat --pid=$processId
}