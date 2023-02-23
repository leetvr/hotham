Write-Output "All your OpenXR are belong to crab"
${Env:RUST_BACKTRACE} = 1

Start-Job -ScriptBlock { cargo run --bin hotham-editor }
Start-Sleep -Seconds 1
cargo run --release --bin hotham_simple_scene_example --features editor
