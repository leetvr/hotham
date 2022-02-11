use ash::vk::{self};
use egui::emath::vec2;
use egui::epaint::Vertex as EguiVertex;
use egui::{CtxRef, Pos2};
use hecs::{Entity, World};
use itertools::izip;
use nalgebra::{vector, Vector3, Vector4};
use rapier3d::prelude::{ColliderBuilder, InteractionGroups};

const BUFFER_SIZE: usize = 1024;

use crate::buffer::Buffer;
use crate::components::mesh::MeshUBO;
use crate::components::{Material, Mesh, Primitive};
use crate::resources::gui_context::SCALE_FACTOR;
use crate::resources::physics_context::PANEL_COLLISION_GROUP;
use crate::resources::{GuiContext, PhysicsContext};
use crate::{
    resources::{RenderContext, VulkanContext},
    texture::Texture,
};
use crate::{vertex::Vertex, COLOR_FORMAT};

use super::{Collider, Transform, TransformMatrix, Visible};

/// A component added to an entity to display a 2D "panel" in space
/// Used by `panels_system`
#[derive(Clone)]
pub struct Panel {
    /// The text to be displayed
    pub text: String,
    /// The dimensions of the panel
    pub extent: vk::Extent2D,
    /// Framebuffer this panel will be written to
    pub framebuffer: vk::Framebuffer,
    /// Vertices of the panel
    pub vertex_buffer: Buffer<EguiVertex>,
    /// Indices of the panel
    pub index_buffer: Buffer<u32>,
    /// Reference to egui context
    pub egui_context: CtxRef,
    /// The raw input for this panel this frame
    pub raw_input: egui::RawInput,
    /// Input received this frame
    pub input: Option<PanelInput>,
    /// A list of buttons in this panel
    pub buttons: Vec<PanelButton>,
}

/// Input to a panel
#[derive(Debug, Clone)]
pub struct PanelInput {
    /// Location of the cursor, in panel space
    pub cursor_location: Pos2,
    /// Value of the controller trigger
    pub trigger_value: f32,
}

/// A button for a panel
#[derive(Debug, Clone)]
pub struct PanelButton {
    /// Text to be displayed
    pub text: String,
    /// Was this button clicked?
    pub clicked_this_frame: bool,
}

impl PanelButton {
    /// Convenience function to create a new panel button
    pub fn new(text: &str) -> Self {
        PanelButton {
            text: text.to_string(),
            clicked_this_frame: false,
        }
    }
}

/// Convenience function to create a panel and add it to a World
pub fn add_panel_to_world(
    text: &str,
    width: u32,
    height: u32,
    buttons: Vec<PanelButton>,
    translation: Vector3<f32>,
    vulkan_context: &VulkanContext,
    render_context: &RenderContext,
    gui_context: &GuiContext,
    physics_context: &mut PhysicsContext,
    world: &mut World,
) -> Entity {
    println!("[PANEL] Adding panel with text {}", text);
    let extent = vk::Extent2D { width, height };
    let output_image = vulkan_context
        .create_image(
            COLOR_FORMAT,
            &extent,
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
    let output_texture = Texture {
        image: output_image,
        sampler,
        descriptor,
    };
    let mesh = create_mesh(&output_texture, &vulkan_context, &render_context, &extent);

    let egui_context = CtxRef::default();
    let raw_input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            Default::default(),
            vec2(extent.width as f32, extent.height as f32) / SCALE_FACTOR,
        )),
        pixels_per_point: Some(SCALE_FACTOR),
        time: Some(0.0),
        ..Default::default()
    };

    let framebuffer = unsafe {
        let attachments = &[output_texture.image.view];
        vulkan_context
            .device
            .create_framebuffer(
                &vk::FramebufferCreateInfo::builder()
                    .render_pass(gui_context.render_pass)
                    .attachments(attachments)
                    .width(extent.width)
                    .height(extent.height)
                    .layers(1),
                None,
            )
            .expect("Failed to create framebuffer.")
    };

    let (vertex_buffer, index_buffer) = create_mesh_buffers(vulkan_context);

    let components = (
        Panel {
            text: text.to_string(),
            extent,
            framebuffer,
            vertex_buffer,
            index_buffer,
            egui_context,
            raw_input,
            input: Default::default(),
            buttons,
        },
        mesh,
        output_texture,
        Transform {
            translation,
            ..Default::default()
        },
        TransformMatrix::default(),
        Visible {},
    );

    let panel_entity = world.spawn(components);
    let (half_width, half_height) = get_panel_dimensions(&extent);
    let collider = ColliderBuilder::cuboid(half_width, half_height, 0.0)
        .sensor(true)
        .collision_groups(InteractionGroups::new(
            PANEL_COLLISION_GROUP,
            PANEL_COLLISION_GROUP,
        ))
        .translation(translation)
        .user_data(panel_entity.id() as _)
        .build();
    let handle = physics_context.colliders.insert(collider);
    let collider = Collider {
        collisions_this_frame: Vec::new(),
        handle,
    };
    world.insert_one(panel_entity, collider).unwrap();
    println!("[PANEL] ..done! {:?}", panel_entity);
    panel_entity
}

fn create_mesh(
    output_texture: &Texture,
    vulkan_context: &VulkanContext,
    render_context: &RenderContext,
    extent: &vk::Extent2D,
) -> Mesh {
    let (material, descriptor_set) =
        get_material(&output_texture, &vulkan_context, &render_context);
    let (half_width, half_height) = get_panel_dimensions(&extent);

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
        indicies_count: 6,
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

pub fn get_panel_dimensions(extent: &vk::Extent2D) -> (f32, f32) {
    let (width, height) = (extent.width as f32, extent.height as f32);
    let (half_width, half_height) = if height > width {
        let half_width = (width / height) * 0.5;
        (half_width, 0.5)
    } else {
        let half_height = (height / width) * 0.5;
        (0.5, half_height)
    };
    (half_width, half_height)
}

fn get_material(
    output_texture: &Texture,
    vulkan_context: &VulkanContext,
    render_context: &RenderContext,
) -> (Material, vk::DescriptorSet) {
    let empty_texture = Texture::empty(&vulkan_context).unwrap();
    // Descriptor set
    let descriptor_set = vulkan_context
        .create_textures_descriptor_sets(
            render_context.descriptor_set_layouts.textures_layout,
            "GUI Texture",
            &output_texture,
            &empty_texture,
            &empty_texture,
            &empty_texture,
            &empty_texture,
        )
        .unwrap()[0];

    let material = Material {
        base_colour_factor: vector![1., 1., 1., 1.],
        emmissive_factor: Vector4::zeros(),
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

fn create_mesh_buffers(vulkan_context: &VulkanContext) -> (Buffer<EguiVertex>, Buffer<u32>) {
    println!("[HOTHAM_DRAW_GUI] Creating mesh buffers..");
    let vertices = (0..BUFFER_SIZE)
        .map(|_| Default::default())
        .collect::<Vec<_>>();
    let empty_index_buffer = [0; BUFFER_SIZE * 2];

    let vertex_buffer = Buffer::new(
        &vulkan_context,
        &vertices,
        vk::BufferUsageFlags::VERTEX_BUFFER,
    )
    .expect("Unable to create font index buffer");

    let index_buffer = Buffer::new(
        &vulkan_context,
        &empty_index_buffer,
        vk::BufferUsageFlags::INDEX_BUFFER,
    )
    .expect("Unable to create font index buffer");

    println!("[HOTHAM_DRAW_GUI] ..done!");

    (vertex_buffer, index_buffer)
}
