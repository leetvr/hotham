${env:RUST_BACKTRACE} = 1

cargo build
if ($?) {
    Set-Location hotham-asteroid
    cargo run

    Write-Output "Hello Asteroid exited with $LASTEXITCODE";
    Pop-Location
}
else {
    Write-Warning "Hotham simulator failed to compile."
}