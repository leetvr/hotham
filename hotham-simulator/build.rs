// Dummy function
#[cfg(target_os = "android")]
fn main() {}

#[cfg(not(target_os = "android"))]
fn main() {
    use shaderc::{Compiler, ShaderKind};
    use std::{fs, path::Path, str};
    println!("cargo:rerun-if-changed=build_input");
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

    let compiler = Compiler::new().expect("Unable to instantiate compiler");

    let input_file = Path::new("./build_input/viewdisplay.vert").to_path_buf();
    let source_text = fs::read(&input_file).expect("Unable to read string");
    let source_text = str::from_utf8(&source_text).expect("Unable to parse string");
    let artifact = compiler
        .compile_into_spirv(
            source_text,
            ShaderKind::Vertex,
            input_file.file_name().unwrap().to_str().unwrap(),
            "main",
            None,
        )
        .expect("Unable to compile file");
    let output_file = Path::new("./src/shaders/viewdisplay.vert.spv").to_path_buf();
    fs::write(&output_file, &artifact.as_binary_u8()).expect("Unable to write spirv to file");

    let input_file = Path::new("./build_input/viewdisplay.frag").to_path_buf();
    let source_text = fs::read(&input_file).expect("Unable to read string");
    let source_text = str::from_utf8(&source_text).expect("Unable to parse string");
    let artifact = compiler
        .compile_into_spirv(
            source_text,
            ShaderKind::Fragment,
            input_file.file_name().unwrap().to_str().unwrap(),
            "main",
            None,
        )
        .expect("Unable to compile file");
    let output_file = Path::new("./src/shaders/viewdisplay.frag.spv").to_path_buf();
    fs::write(&output_file, &artifact.as_binary_u8()).expect("Unable to write spirv to file");
}
