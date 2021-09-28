adb shell am force-stop rust.beat_saber_example

Set-Location examples\beat-saber-clone
cargo apk run --release

if ($?) {
    Start-Sleep -Seconds 2
    $processIdStr = (adb shell pidof rust.beat_saber_example) | Out-String
    Write-Output $processIdStr
    $processId = $processIdStr -as [int]
    Write-Output $processId
    adb logcat --pid=$processId
    Pop-Location
}
