${env:RUST_BACKTRACE} = 1

if ($?) {
    cd ../../
    cargo run --bin hotham_beat_saber_example --release
}