Write-Output "CUBES"
${env:RUST_BACKTRACE} = 1

try {
    Invoke-Command -ErrorAction Stop -ScriptBlock { cargo build }
    Push-Location -Path "C:\Users\kanem\Development\hotham-simulator"
    Invoke-Command -ErrorAction Stop -ScriptBlock { cargo build }
    Push-Location -Path "C:\Users\kanem\Development\hotham-cubeworld"
    cargo run
}
catch {
    Write-Warning "Problem!"
}
finally {
    Push-Location -Path "C:\Users\kanem\Development\hotham"
}