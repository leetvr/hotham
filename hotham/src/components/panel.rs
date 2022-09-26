use ash::vk::{self};
use egui::Pos2;
use itertools::izip;
use nalgebra::{vector, Vector2};

use crate::components::Mesh;
use crate::hotham_error::HothamError;
use crate::rendering::material::Material;
use crate::rendering::mesh_data::MeshData;
use crate::rendering::primitive::Primitive;
use crate::rendering::vertex::Vertex;
use crate::{
    contexts::{RenderContext, VulkanContext},
    rendering::texture::Texture,
};

pub struct Panel {
    /// The resolution of the Panel
    pub resolution: vk::Extent2D,
    /// The world-size of the Panel
    pub world_size: Vector2<f32>,
    /// Texture backing the Panel
    pub texture: Texture,
    /// Input received this frame
    pub input: Option<PanelInput>,
}

impl Panel {
    pub fn create(
        vulkan_context: &VulkanContext,
        render_context: &mut RenderContext,
        resolution: vk::Extent2D,
        world_size: Vector2<f32>,
    ) -> Result<(Panel, Mesh), HothamError> {
        let texture = Texture::empty(vulkan_context, render_context, resolution);
        let mesh = create_panel_mesh(&texture, render_context, world_size);

        Ok((
            Panel {
                resolution,
                world_size,
                texture,
                input: Default::default(),
            },
            mesh,
        ))
    }
}

fn create_panel_mesh(
    output_texture: &Texture,
    render_context: &mut RenderContext,
    world_size: Vector2<f32>,
) -> Mesh {
    let material_id = add_material(output_texture, render_context);
    let (half_width, half_height) = (world_size.x / 2., world_size.y / 2.);

    let positions = [
        vector![-half_width, half_height, 0.],  // v0
        vector![half_width, -half_height, 0.],  // v1
        vector![half_width, half_height, 0.],   // v2
        vector![-half_width, -half_height, 0.], // v3
    ];
    let tex_coords_0 = [
        vector![0., 0.], // v0
        vector![1., 1.], // v1
        vector![1., 0.], // v2
        vector![0., 1.], // v3
    ];
    let vertices: Vec<Vertex> = izip!(positions, tex_coords_0)
        .into_iter()
        .map(|(p, t)| Vertex {
            position: p,
            texture_coords: t,
            ..Default::default()
        })
        .collect();

    let indices = [0, 1, 2, 0, 3, 1];
    let primitive = Primitive::new(&vertices, &indices, material_id, render_context);
    Mesh::new(MeshData::new(vec![primitive]), render_context)
}

fn add_material(output_texture: &Texture, render_context: &mut RenderContext) -> u32 {
    let mut material = Material::unlit_white();
    material.base_color_texture_set = output_texture.index;
    unsafe { render_context.resources.materials_buffer.push(&material) }
}

/// Input to a panel
#[derive(Debug, Clone)]
pub struct PanelInput {
    /// Location of the cursor, in panel space
    pub cursor_location: Pos2,
    /// Value of the controller trigger
    pub trigger_value: f32,
}
