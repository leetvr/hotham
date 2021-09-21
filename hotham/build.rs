use std::fs;

use shaderc::{Compiler, ShaderKind};

fn main() {
    println!("cargo:rerun-if-changed=./src/shaders");
    eprintln!("HOTHAM BUILD IS RUNNING");

    for path in fs::read_dir("./src/shaders").unwrap() {
        let path = path.unwrap().path();
        let ext = path.extension().unwrap();
        let mut compiler = shaderc::Compiler::new().unwrap();

        if ext == "spv" {
            continue;
        }

        if ext == "frag" {
            compile_shader(path, &mut compiler, ShaderKind::Fragment);
        } else if ext == "vert" {
            compile_shader(path, &mut compiler, ShaderKind::Vertex);
        }
    }
}

fn compile_shader(
    path: std::path::PathBuf,
    compiler: &mut Compiler,
    shader_kind: ShaderKind,
) -> () {
    let artifact = {
        let input_file_name = path.file_name().unwrap().to_str().unwrap();
        let source_text = fs::read_to_string(&path).unwrap();
        compiler.compile_into_spirv(&source_text, shader_kind, input_file_name, "main", None)
    }
    .unwrap();

    let extension = path.extension().unwrap().to_string_lossy();
    let mut output_path = path.clone();
    output_path.set_extension(format!("{}.spv", extension));
    let output_path = format!(
        "./shaders/{}",
        output_path.file_name().unwrap().to_str().unwrap()
    );
    println!(
        "Combining {:?} at {:?} to {:?}",
        shader_kind, path, output_path
    );
    fs::write(output_path, artifact.as_binary_u8()).unwrap();
}
