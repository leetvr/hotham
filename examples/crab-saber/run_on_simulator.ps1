${env:RUST_BACKTRACE} = 1

if ($?) {
    cd ../../
    cargo run --bin hotham_crab_saber --release
}