Write-Output "All your OpenXR are belong to crab"
${Env:RUST_BACKTRACE} = 1

try {
    cargo build -p hotham_openxr_client
    cargo run --bin hotham_simple_scene_example
}
catch {
    Write-Warning "Problem!"
}
finally {
    Pop-Location
}
