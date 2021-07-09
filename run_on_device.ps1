${env:RUST_BACKTRACE} = 1
adb shell am force-stop rust.hotham_asteroid

Set-Location hotham-asteroid
cargo apk run

if ($?) {
    Start-Sleep -Seconds 2
    $processIdStr = (adb shell pidof rust.hotham_asteroid) | Out-String
    Write-Output $processIdStr
    $processId = $processIdStr -as [int]
    Write-Output $processId
    adb logcat --pid=$processId
}


Set-Location C:\Users\kanem\Development\hotham