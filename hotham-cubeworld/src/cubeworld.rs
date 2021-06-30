use std::{io::Cursor};

use cgmath::{vec2, vec3};
use hotham::{Program, ProgramInitialization, Vertex, read_spv_from_bytes, HothamResult as Result};
use image::{GenericImageView};
use itertools::izip;

#[derive(Debug, Clone)]
pub struct Cubeworld {
    model_data: ModelData,
    needs_update: bool,
    cube_count: u32,
}

// const CUBE_VERTICES: u32 = 8;

impl Cubeworld {
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


impl Program for Cubeworld {
    fn init(&mut self) -> Result<ProgramInitialization> {
        // TODO: This should be somehow relative to hotham-cubeworld already
        let vertex_shader = read_spv_from_bytes(&mut Cursor::new(include_bytes!("shaders/cube.vert.spv")))?;
        let fragment_shader = read_spv_from_bytes(&mut Cursor::new(include_bytes!("shaders/cube.frag.spv")))?;
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

#[derive(Debug, Clone)]
struct Cube {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
}

impl Default for Cube {
    #[rustfmt::skip]
    fn default() -> Self {
        let v0 =vec3(-1.0, -1.0, 1.0);
        let v1 = vec3(-1.0, 1.0, 1.0);
        let v2 = vec3(1.0, 1.0, 1.0);

        let v3 = vec3(1.0, -1.0, 1.0);
        let v4 = vec3(-1.0, -1.0, -1.0);
        let v5 = vec3(1.0, -1.0, -1.0);
        
        let v6 = vec3(1.0, 1.0, -1.0);
        let v7 = vec3(-1.0, 1.0, -1.0);

        let positions = vec![
            v0, v1, v2,
            v3, v4, v5,
            v6, v7
        ];

        let indices = vec![
            0, 1, 2, 2, 3, 0, // FRONT
            0, 3, 4, 4, 3, 5, // TOP
            5, 6, 4, 4, 6, 7, // BACK
            7, 1, 4, 4, 1, 0, // LEFT
            1, 7, 2, 2, 7, 6, // BOTTOM
            2, 6, 3, 3, 6, 5, // RIGHT
        ];

        let vertices = positions.into_iter().map(Vertex::pos).collect();
        Self { vertices, indices }
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
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()..buffer.index() + buffer.length()]));
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

    let vertices = izip!(positions, colours, tex_coords, normals).into_iter().map(Vertex::from_zip).collect();

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
        let mut cubeworld = Cubeworld::new();
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