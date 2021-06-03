Write-Output "CUBES"

try {
    cargo build
    Push-Location -Path "C:\Users\kanem\Development\hotham-cubeworld"
    cargo run
}
catch {
    Write-Warning "Problem!"
}
finally {
    Pop-Location
}