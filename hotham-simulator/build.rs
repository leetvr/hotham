use std::{fs, path::Path, str};

use shaderc::{Compiler, ShaderKind};

#[cfg(target_os = "windows")]
fn main() {
    println!("cargo:rerun-if-changed=src/shaders/*.frag");
    println!("cargo:rerun-if-changed=src/shaders/*.vert");
    println!("cargo:rerun-if-changed=wrapper.h");
    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header("wrapper.h")
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    bindings
        .write_to_file("src/bindings.rs")
        .expect("Couldn't write bindings!");

    let mut compiler = Compiler::new().expect("Unable to instantiate compiler");

    let mut file = Path::new("./src/shaders/viewdisplay.vert").to_path_buf();
    let source_text = fs::read(&file).expect("Unable to read string");
    let source_text = str::from_utf8(&source_text).expect("Unable to parse string");
    let artifact = compiler
        .compile_into_spirv(
            source_text,
            ShaderKind::Vertex,
            file.file_name().unwrap().to_str().unwrap(),
            "main",
            None,
        )
        .expect("Unable to compile file");
    file.set_extension("vert.spv");
    fs::write(&file, &artifact.as_binary_u8()).expect("Unable to write spirv to file");

    let mut file = Path::new("./src/shaders/viewdisplay.frag").to_path_buf();
    let source_text = fs::read(&file).expect("Unable to read string");
    let source_text = str::from_utf8(&source_text).expect("Unable to parse string");
    let artifact = compiler
        .compile_into_spirv(
            source_text,
            ShaderKind::Fragment,
            file.file_name().unwrap().to_str().unwrap(),
            "main",
            None,
        )
        .expect("Unable to compile file");
    file.set_extension("frag.spv");
    fs::write(&file, &artifact.as_binary_u8()).expect("Unable to write spirv to file");
}
