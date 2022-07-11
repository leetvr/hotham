pub fn main() {
    // On Android, we must ensure that we're dynamically linking against the C++ standard library.
    // For more details, see https://github.com/rust-windowing/android-ndk-rs/issues/167
    println!("cargo:rustc-link-lib=dylib=c++");
}
