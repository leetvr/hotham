pub fn main() {
    // On Android, we must ensure that we're dynamically linking against the C++ standard library.
    // For more details, see https://github.com/rust-windowing/android-ndk-rs/issues/167
    use std::env::var;
    if var("TARGET")
        .map(|target| target == "aarch64-linux-android")
        .unwrap_or(false)
    {
        println!("cargo:rustc-link-lib=dylib=c++");
    }
}
