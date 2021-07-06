use std::{
    fs::OpenOptions,
    io::Cursor,
    sync::{Arc, Mutex},
};

use cgmath::{vec2, vec3};
use hotham::{read_spv_from_bytes, HothamResult as Result, Program, ProgramInitialization, Vertex};
use itertools::izip;
use libktx_rs::{sources::StreamSource, RustKtxStream, TextureCreateFlags, TextureSource};

#[derive(Debug, Clone)]
pub struct Asteroid {
    model_data: ModelData,
    needs_update: bool,
    cube_count: u32,
}

impl Asteroid {
    pub fn new() -> Self {
        Self {
            model_data: Default::default(),
            needs_update: true,
            cube_count: 1,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ModelData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub image_buf: Vec<u8>,
    pub image_height: u32,
    pub image_width: u32,
}

impl Program for Asteroid {
    fn init(&mut self) -> Result<ProgramInitialization> {
        let vertex_shader = read_spv_from_bytes(&mut Cursor::new(include_bytes!(
            "shaders/asteroid.vert.spv"
        )))?;
        let fragment_shader = read_spv_from_bytes(&mut Cursor::new(include_bytes!(
            "shaders/asteroid.frag.spv"
        )))?;
        self.model_data = load_model_from_gltf_optimized();

        Ok(ProgramInitialization {
            vertices: &self.model_data.vertices,
            indices: &self.model_data.indices,
            vertex_shader,
            fragment_shader,
            image_width: self.model_data.image_width,
            image_height: self.model_data.image_height,
            image_buf: self.model_data.image_buf.clone(),
        })
    }

    fn update(&mut self) -> (&Vec<Vertex>, &Vec<u32>) {
        (&self.model_data.vertices, &self.model_data.indices)
    }
}

pub fn load_model_from_gltf_optimized() -> ModelData {
    let gtlf_buf = Cursor::new(include_bytes!("../assets/asteroid.gltf"));
    let buffers = include_bytes!("../assets/asteroid_data.bin");
    let gltf = gltf::Gltf::from_reader(gtlf_buf).unwrap();
    let mut indices = Vec::new();

    let mut positions = Vec::new();
    let mut tex_coords = Vec::new();
    let mut colours = Vec::new();
    let mut normals = Vec::new();

    for mesh in gltf.meshes() {
        for primitive in mesh.primitives() {
            let reader = primitive
                .reader(|buffer| Some(&buffers[buffer.index()..buffer.index() + buffer.length()]));
            if let Some(iter) = reader.read_positions() {
                for v in iter {
                    positions.push(vec3(v[0], -v[1], v[2]));
                }
            }
            if let Some(iter) = reader.read_normals() {
                for v in iter {
                    normals.push(vec3(v[0], -v[1], v[2]));
                }
            }
            if let Some(iter) = reader.read_tex_coords(0) {
                for v in iter.into_f32() {
                    tex_coords.push(vec2(v[0], v[1]));
                }
            }
            if let Some(iter) = reader.read_colors(0) {
                for v in iter.into_rgb_f32() {
                    colours.push(vec3(v[0], v[1], v[2]));
                }
            }
            if let Some(iter) = reader.read_indices() {
                for i in iter.into_u32() {
                    indices.push(i);
                }
            }
        }
    }

    let vertices = izip!(positions, colours, tex_coords, normals)
        .into_iter()
        .map(Vertex::from_zip)
        .collect();

    let mut images = vec![
        parse_ktx("C:\\Users\\kanem\\Development\\hotham\\hotham-asteroid\\assets\\asteroid_optimized_img0.ktx2"),
        parse_ktx("C:\\Users\\kanem\\Development\\hotham\\hotham-asteroid\\assets\\asteroid_optimized_img1.ktx2"),
        parse_ktx("C:\\Users\\kanem\\Development\\hotham\\hotham-asteroid\\assets\\asteroid_optimized_img2.ktx2"),
        parse_ktx("C:\\Users\\kanem\\Development\\hotham\\hotham-asteroid\\assets\\asteroid_optimized_img3.ktx2"),
        parse_ktx("C:\\Users\\kanem\\Development\\hotham\\hotham-asteroid\\assets\\asteroid_optimized_img4.ktx2"),
    ];

    let (image_buf, image_height, image_width) = images.remove(2);

    ModelData {
        vertices,
        indices,
        image_buf,
        image_height,
        image_width,
    }
}

pub fn parse_ktx(path: &str) -> (Vec<u8>, u32, u32) {
    let file = Box::new(
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .unwrap(),
    );
    let stream = RustKtxStream::new(file).unwrap();
    let source = Arc::new(Mutex::new(stream));
    let texture = StreamSource::new(source, TextureCreateFlags::LOAD_IMAGE_DATA)
        .create_texture()
        .unwrap();

    let image_buf = texture.data().to_vec();
    let (image_height, image_width, _size) = unsafe {
        let ktx_texture = texture.handle();
        (
            (*ktx_texture).baseHeight,
            (*ktx_texture).baseWidth,
            (*ktx_texture).dataSize,
        )
    };

    // assert_eq!(texture.get_image_size(0).unwrap(), size);

    (image_buf, image_width, image_height)
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;
    #[test]
    pub fn test_program_init() {
        let tick = Instant::now();
        let mut cubeworld = Asteroid::new();
        let init = cubeworld.init().unwrap();
        let tock = Instant::now();
        let elapsed = (tock - tick).as_millis();
        println!("Took {}", elapsed);
        assert_eq!(init.vertices.len(), 618);
        assert_eq!(init.indices.len(), 25728);
        assert_eq!(init.image_height, 2048);
        assert_eq!(init.image_width, 2048);
    }
}
