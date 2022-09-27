adb shell am force-stop rust.complex_scene_example

Set-Location $PSScriptRoot\..
cargo apk run --release

if ($?) {
    Start-Sleep -Seconds 2
    $processIdStr = (adb shell pidof rust.complex_scene_example) | Out-String
    Write-Output $processIdStr
    $processId = $processIdStr -as [int]
    Write-Output $processId
    adb logcat --pid=$processId
}
