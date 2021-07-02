use std::io::Cursor;

use cgmath::{vec2, vec3};
use hotham::{read_spv_from_bytes, HothamResult as Result, Program, ProgramInitialization, Vertex};
use image::GenericImageView;
use itertools::izip;

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
struct ModelData {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    image_buf: Vec<u8>,
    image_height: u32,
    image_width: u32,
}

impl Program for Asteroid {
    fn init(&mut self) -> Result<ProgramInitialization> {
        let vertex_shader = read_spv_from_bytes(&mut Cursor::new(include_bytes!(
            "shaders/asteroid.vert.spv"
        )))?;
        let fragment_shader = read_spv_from_bytes(&mut Cursor::new(include_bytes!(
            "shaders/asteroid.frag.spv"
        )))?;
        self.model_data = load_model();

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

fn load_model() -> ModelData {
    println!("Loading model..");
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

    let mut cursor = Cursor::new(include_bytes!("../assets/asteroid_img2.png"));
    let mut reader = image::io::Reader::new(&mut cursor);
    reader.set_format(image::ImageFormat::Png);
    let img = reader.decode().unwrap();
    let image_width = img.width();
    let image_height = img.height();
    let image_buf = img.into_rgba8().into_raw();

    // let image_buf = Vec::new();
    // let image_height = 0;
    // let image_width = 0;

    println!("..done!");

    ModelData {
        vertices,
        indices,
        image_buf,
        image_height,
        image_width,
    }
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
        assert_eq!(init.indices.len(), 1800);
        assert_eq!(init.image_height, 2048);
        assert_eq!(init.image_width, 2048);
    }
}
