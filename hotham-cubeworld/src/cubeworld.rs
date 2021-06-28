use std::{io::Cursor};

use cgmath::{vec3};
use hotham::{Program, ProgramInitialization, Vertex, read_spv_from_bytes, HothamResult as Result};

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

        let cube = Cube::default();
        self.vertices = cube.vertices;
        self.indices = cube.indices;

        self.needs_update = false;
    }
}


impl Program for Cubeworld {
    fn init(&mut self) -> Result<ProgramInitialization> {
        // TODO: This should be somehow relative to hotham-cubeworld already
        let vertex_shader = read_spv_from_bytes(&mut Cursor::new(include_bytes!("shaders/cube.vert.spv")))?;
        let fragment_shader = read_spv_from_bytes(&mut Cursor::new(include_bytes!("shaders/cube.frag.spv")))?;
        let img = image::io::Reader::open("./src/tutorials/images/viking_room.png")
            .expect("Unable to read image")
            .decode()
            .expect("Unable to read image")
            .to_rgba8();
        let image_width = img.width();
        let image_height = img.height();
        let image_buf = img.into_raw();

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