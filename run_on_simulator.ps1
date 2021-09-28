${env:RUST_BACKTRACE} = 1

cargo build -p hotham-simulator
if ($?) {
    cargo run simple-scene-example --release
}