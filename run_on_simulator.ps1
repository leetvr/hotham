${env:RUST_BACKTRACE} = 1

if ($?) {
    cargo run --bin simple_scene_example --release
}