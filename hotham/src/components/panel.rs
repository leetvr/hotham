use ash::vk::{self};
use egui::Pos2;
use itertools::izip;
use nalgebra::{vector, Vector2, Vector4};

use crate::components::mesh::MeshUBO;
use crate::components::{Material, Mesh, Primitive};
use crate::hotham_error::HothamError;
use crate::rendering::buffer::Buffer;
use crate::{
    rendering::texture::Texture,
    resources::{RenderContext, VulkanContext},
};
use crate::{rendering::vertex::Vertex, COLOR_FORMAT};

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
        render_context: &RenderContext,
        resolution: vk::Extent2D,
        world_size: Vector2<f32>,
    ) -> Result<(Panel, Mesh), HothamError> {
        let output_image = vulkan_context
            .create_image(
                COLOR_FORMAT,
                &resolution,
                vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
                1,
                1,
            )
            .unwrap();
        let sampler = vulkan_context
            .create_texture_sampler(vk::SamplerAddressMode::REPEAT, 1)
            .unwrap();
        let descriptor = vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(output_image.view)
            .sampler(sampler)
            .build();
        let texture = Texture {
            image: output_image,
            sampler,
            descriptor,
        };
        let mesh = create_mesh(&texture, vulkan_context, render_context, world_size);

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

fn create_mesh(
    output_texture: &Texture,
    vulkan_context: &VulkanContext,
    render_context: &RenderContext,
    world_size: Vector2<f32>,
) -> Mesh {
    let (material, descriptor_set) = get_material(output_texture, vulkan_context, render_context);
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
            texture_coords_0: t,
            ..Default::default()
        })
        .collect();

    let vertex_buffer = Buffer::new(
        vulkan_context,
        &vertices,
        vk::BufferUsageFlags::VERTEX_BUFFER,
    )
    .unwrap();

    let index_buffer = Buffer::new(
        vulkan_context,
        &[0, 1, 2, 0, 3, 1],
        vk::BufferUsageFlags::INDEX_BUFFER,
    )
    .unwrap();

    let primitive = Primitive {
        index_buffer,
        vertex_buffer,
        indices_count: 6,
        material,
        texture_descriptor_set: descriptor_set,
    };

    // Create descriptor sets
    let descriptor_sets = vulkan_context
        .create_mesh_descriptor_sets(render_context.descriptor_set_layouts.mesh_layout, "GUI")
        .unwrap();
    let descriptor_sets = [descriptor_sets[0]];

    let mesh_ubo = MeshUBO::default();
    let ubo_buffer = Buffer::new(
        vulkan_context,
        &[mesh_ubo],
        vk::BufferUsageFlags::UNIFORM_BUFFER,
    )
    .unwrap();
    vulkan_context.update_buffer_descriptor_set(
        &ubo_buffer,
        descriptor_sets[0],
        0,
        vk::DescriptorType::UNIFORM_BUFFER,
    );

    Mesh {
        descriptor_sets,
        ubo_buffer,
        ubo_data: mesh_ubo,
        primitives: vec![primitive],
    }
}

fn get_material(
    output_texture: &Texture,
    vulkan_context: &VulkanContext,
    render_context: &RenderContext,
) -> (Material, vk::DescriptorSet) {
    let empty_texture = Texture::empty(vulkan_context).unwrap();
    // Descriptor set
    let descriptor_set = vulkan_context
        .create_textures_descriptor_sets(
            render_context.descriptor_set_layouts.textures_layout,
            "GUI Texture",
            &[
                output_texture,
                &empty_texture,
                &empty_texture,
                &empty_texture,
                &empty_texture,
            ],
        )
        .unwrap()[0];

    let material = Material {
        base_color_factor: vector![1., 1., 1., 1.],
        emissive_factor: Vector4::zeros(),
        diffuse_factor: Vector4::zeros(),
        specular_factor: Vector4::zeros(),
        workflow: 2.,
        base_color_texture_set: 0,
        metallic_roughness_texture_set: -1,
        normal_texture_set: -1,
        occlusion_texture_set: -1,
        emissive_texture_set: -1,
        metallic_factor: 0.,
        roughness_factor: 0.,
        alpha_mask: 0.,
        alpha_mask_cutoff: 1.,
    };

    (material, descriptor_set)
}

/// Input to a panel
#[derive(Debug, Clone)]
pub struct PanelInput {
    /// Location of the cursor, in panel space
    pub cursor_location: Pos2,
    /// Value of the controller trigger
    pub trigger_value: f32,
}
