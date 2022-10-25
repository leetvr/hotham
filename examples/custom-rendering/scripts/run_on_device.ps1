adb shell am force-stop rust.custom_rendering_example

Set-Location $PSScriptRoot\..
cargo apk run --release

if ($?) {
    Start-Sleep -Seconds 2
    $processIdStr = (adb shell pidof rust.custom_rendering_example) | Out-String
    Write-Output $processIdStr
    $processId = $processIdStr -as [int]
    Write-Output $processId
    adb logcat --pid=$processId
}
