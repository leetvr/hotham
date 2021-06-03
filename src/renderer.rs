use crate::{vulkan_context::VulkanContext, Vertex};

#[derive(Clone, Debug, Default)]
pub(crate) struct Renderer {
    context: VulkanContext,
}

impl Renderer {
    pub(crate) fn new(context: VulkanContext) -> Self {
        Self { context }
    }

    pub fn update(&self, vertices: &Vec<Vertex>, indices: &Vec<u32>) -> () {
        println!("Vertices are now: {:?}", vertices);
        println!("Indices are now: {:?}", indices);
    }
}
