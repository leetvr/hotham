Write-Output "All your OpenXR are belong to crab"
${Env:RUST_BACKTRACE} = 1

Write-Output "Starting editor.."
Start-Job -ScriptBlock { cargo run --bin hotham-editor }

Write-Output "Sleeping.."
Start-Sleep -Seconds 5

Write-Output "Starting game.."
cargo run --release --bin hotham_simple_scene_example --features editor
