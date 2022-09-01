adb shell am force-stop rust.crab_saber

Set-Location $PSScriptRoot\..
cargo apk run --release

if ($?) {
    $processId = $null
    foreach ($i in 1..5) {
        $processId = adb shell pidof rust.crab_saber
        if ($processId) { break }
        Write-Output "Waiting for process to start, sleeping..."
        Start-Sleep -Seconds 1
    }
    if ($processId) {
        Write-Output "Found PID of " $processId
        adb logcat --pid=$processId
    } else {
        Write-Error "Failed to find PID of rust.crab_saber"
    }
}
