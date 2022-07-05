// Dummy function
#[cfg(target_os = "android")]
fn main() {}

#[cfg(not(target_os = "android"))]
fn main() {
    println!("cargo:rerun-if-changed=build_input");
    // use std::{fs, path::Path, str};
    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.

    // Annoying as requires complicated clang setup on windows. Headers are unlikely to change any time soon.
    // let bindings = bindgen::Builder::default()
    //     // The input header we would like to generate
    //     // bindings for.
    //     .header("build_input/wrapper.h")
    //     // Finish the builder and generate the bindings.
    //     .generate()
    //     // Unwrap the Result and panic on failure.
    //     .expect("Unable to generate bindings");

    // bindings
    //     .write_to_file("src/bindings.rs")
    //     .expect("Couldn't write bindings!");
}
