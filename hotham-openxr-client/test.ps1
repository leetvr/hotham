Write-Output "All your OpenXR are belong to crab"
${Env:RUST_BACKTRACE} = 1

cargo build -p hotham_openxr_client
# Start-Job -ScriptBlock { cargo run --bin hotham-editor }
# Start-Sleep -seconds 1
cargo run --bin hotham_simple_scene_example
