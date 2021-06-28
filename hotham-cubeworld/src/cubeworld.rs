use std::{io::Cursor};

use cgmath::{vec2, vec3};
use hotham::{Program, ProgramInitialization, Vertex, read_spv_from_bytes, HothamResult as Result};
use image::{GenericImageView, ImageFormat::Tga};
use tobj::LoadOptions;

#[derive(Debug, Clone)]
pub struct Cubeworld {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    needs_update: bool,
    cube_count: u32,
}

// const CUBE_VERTICES: u32 = 8;

impl Cubeworld {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            needs_update: true,
            cube_count: 1,
        }
    }

    fn update_vertices(&mut self) {
        self.vertices.clear();
        self.indices.clear();

        let (vertices, indices) = load_model();
        self.vertices = vertices;
        self.indices = indices;


        self.needs_update = false;
    }
}


impl Program for Cubeworld {
    fn init(&mut self) -> Result<ProgramInitialization> {
        // TODO: This should be somehow relative to hotham-cubeworld already
        let vertex_shader = read_spv_from_bytes(&mut Cursor::new(include_bytes!("shaders/cube.vert.spv")))?;
        let fragment_shader = read_spv_from_bytes(&mut Cursor::new(include_bytes!("shaders/cube.frag.spv")))?;
        let mut cursor = Cursor::new(include_bytes!("../assets/asteroid.tga"));
        let mut reader = image::io::Reader::new(&mut cursor);
        reader.set_format(Tga);
        let img = reader.decode().unwrap();
        let image_width = img.width();
        let image_height = img.height();
        let image_buf = img.into_bytes();

        let (vertices, indices) = self.update();

        Ok(ProgramInitialization {
            vertices,
            indices,
            vertex_shader,
            fragment_shader,
            image_width,
            image_height,
            image_buf
        })
    }

    fn update(&mut self) -> (&Vec<Vertex>, &Vec<u32>) {
        if self.needs_update {
            self.update_vertices();
        }

        (&self.vertices, &self.indices)
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

fn load_model() -> (Vec<Vertex>, Vec<u32>) {
    println!("Loading model..");
    let obj_buf = &mut Cursor::new(include_bytes!("../assets/asteroid.obj"));
    let (models, _) = tobj::load_obj_buf(obj_buf, &LoadOptions { single_index: true, triangulate: true, ..Default::default()}, |_| {
        let mtl_buf = &mut Cursor::new(include_bytes!("../assets/asteroid.mtl"));
        tobj::load_mtl_buf(mtl_buf)
    }).unwrap();
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for model in models {
        for index in model.mesh.indices {
            indices.push(indices.len() as u32);
            let i = index as usize;

            let v1 = model.mesh.positions[i * 3];
            let v2 = model.mesh.positions[i * 3 + 1];
            let v3 = model.mesh.positions[i * 3 + 2];
            let pos = vec3(v1, v2, v3);

            let t1 = model.mesh.texcoords[i * 2];
            let t2 = model.mesh.texcoords[i * 2 + 1];
            let texture_coordinate = vec2(t1, 1.0 - t2);

            let colour = vec3(1.0, 1.0, 1.0);
            let vertex = Vertex::new(pos, colour, texture_coordinate);
            vertices.push(vertex)
        }
    }

    println!("..done!");

    (vertices, indices)
}