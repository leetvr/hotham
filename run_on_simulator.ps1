${env:RUST_BACKTRACE} = 1

cargo build -p hotham-simulator
if ($?) {
    cargo run -p hotham-asteroid

    Write-Output "HelloXR exited with $LASTEXITCODE";
}
else {
    Write-Warning "Hotham simulator failed to compile."
}