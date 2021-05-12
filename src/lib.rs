use std::{path::Path, thread::sleep, time::Duration};

use cgmath::Vector3;

#[derive(Clone, Debug)]
pub struct App<P: Program> {
    program: P,
    should_quit: bool,
    renderer: Renderer,
}

#[derive(Clone, Debug, Default)]
struct Renderer {}

#[derive(Clone, Debug)]
pub struct Vertex {
    position: Vector3<f32>,
    color: Vector3<f32>,
}

impl Vertex {
    pub fn new(position: Vector3<f32>, color: Vector3<f32>) -> Self {
        Self { position, color }
    }
}

impl Renderer {
    pub fn update(&self, vertices: &Vec<Vertex>, indices: &Vec<u32>) -> () {
        println!("Vertices are now: {:?}", vertices);
        println!("Indices are now: {:?}", indices);
    }
}

impl<P> App<P>
where
    P: Program,
{
    pub fn new(program: P) -> Self {
        let params = program.init();
        println!("Initialised program with {:?}", params);
        Self {
            program,
            renderer: Default::default(),
            should_quit: false,
        }
    }

    pub fn run(&self) -> () {
        while !&self.should_quit {
            // Tell the program to update its geometry
            // It's unclear what the output of "update" should be. Perhaps it should just be a reference to a Vector of vertices?
            // These vertices can then be pushed back into the Vulkan pipeline.
            // So essentially App is the glue between the developer's program and our internal Renderer
            let (vertices, indices) = self.program.update();
            self.renderer.update(vertices, indices);
            sleep(Duration::from_secs(1))
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProgramInitialization<'a> {
    pub vertices: &'a Vec<Vertex>,
    pub indices: &'a Vec<u32>,
    pub vertex_shader: &'a Path,
    pub fragment_shader: &'a Path,
}

pub trait Program {
    fn update(&self) -> (&Vec<Vertex>, &Vec<u32>);
    fn init(&self) -> ProgramInitialization;
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
