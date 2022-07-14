adb shell am force-stop rust.multiple_guis

Set-Location $PSScriptRoot\..
cargo apk run --release

if ($?) {
    Start-Sleep -Seconds 2
    $processIdStr = (adb shell pidof rust.multiple_guis) | Out-String
    Write-Output $processIdStr
    $processId = $processIdStr -as [int]
    Write-Output $processId
    adb logcat --pid=$processId
}
