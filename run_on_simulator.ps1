${env:RUST_BACKTRACE} = 1

if ($?) {
    cargo run --bin hotham_beat_saber_example --release
}