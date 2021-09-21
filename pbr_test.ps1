cargo test -p hotham test_rendering_pbr --release -- --nocapture
if ($?) {
    code -r hotham\*.jpeg
}