use ash::vk::{self};
use egui::emath::vec2;
use egui::epaint::Vertex as EguiVertex;
use egui::CtxRef;
use hecs::{Entity, World};
use nalgebra::{Vector2, Vector3};
use rapier3d::prelude::{ColliderBuilder, InteractionGroups};

const BUFFER_SIZE: usize = 1024;

use crate::buffer::Buffer;
use crate::components::Panel;
use crate::resources::gui_context::SCALE_FACTOR;
use crate::resources::physics_context::PANEL_COLLISION_GROUP;
use crate::resources::{GuiContext, PhysicsContext};
use crate::resources::{RenderContext, VulkanContext};

use super::{Collider, Transform, TransformMatrix, Visible};

/// A component added to an entity to display a 2D "panel" in space
/// Used by `panels_system`
#[derive(Clone)]
pub struct UIPanel {
    /// The text to be displayed
    pub text: String,
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
    /// A list of buttons in this panel
    pub buttons: Vec<UIPanelButton>,
}

/// A button for a panel
#[derive(Debug, Clone)]
pub struct UIPanelButton {
    /// Text to be displayed
    pub text: String,
    /// Was this button hovered?
    pub hovered_last_frame: bool,
    pub hovered_this_frame: bool,
    /// Was this button clicked?
    pub clicked_this_frame: bool,
}

impl UIPanelButton {
    /// Convenience function to create a new panel button
    pub fn new(text: &str) -> Self {
        UIPanelButton {
            text: text.to_string(),
            hovered_last_frame: false,
            hovered_this_frame: false,
            clicked_this_frame: false,
        }
    }
}

/// Convenience function to create a panel and add it to a World
#[cfg_attr(feature = "cargo-clippy", allow(clippy::too_many_arguments))]
pub fn add_ui_panel_to_world(
    text: &str,
    resolution: vk::Extent2D,
    world_size: Vector2<f32>,
    translation: Vector3<f32>,
    buttons: Vec<UIPanelButton>,
    vulkan_context: &VulkanContext,
    render_context: &RenderContext,
    gui_context: &GuiContext,
    physics_context: &mut PhysicsContext,
    world: &mut World,
) -> Entity {
    println!("[PANEL] Adding panel with text {}", text);
    let (panel, mesh) = Panel::create(vulkan_context, render_context, resolution, world_size)
        .expect("failed to create Panel");
    let egui_context = CtxRef::default();
    let raw_input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            Default::default(),
            vec2(resolution.width as f32, resolution.height as f32) / SCALE_FACTOR,
        )),
        pixels_per_point: Some(SCALE_FACTOR),
        time: Some(0.0),
        ..Default::default()
    };

    let framebuffer = unsafe {
        let attachments = &[panel.texture.image.view];
        vulkan_context
            .device
            .create_framebuffer(
                &vk::FramebufferCreateInfo::builder()
                    .render_pass(gui_context.render_pass)
                    .attachments(attachments)
                    .width(resolution.width)
                    .height(resolution.height)
                    .layers(1),
                None,
            )
            .expect("Failed to create framebuffer.")
    };

    let (vertex_buffer, index_buffer) = create_mesh_buffers(vulkan_context);

    let components = (
        panel,
        mesh,
        UIPanel {
            text: text.to_string(),
            framebuffer,
            vertex_buffer,
            index_buffer,
            egui_context,
            raw_input,
            buttons,
        },
        Transform {
            translation,
            ..Default::default()
        },
        TransformMatrix::default(),
        Visible {},
    );

    let panel_entity = world.spawn(components);
    let (half_width, half_height) = (world_size.x / 2., world_size.y / 2.);
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

fn create_mesh_buffers(vulkan_context: &VulkanContext) -> (Buffer<EguiVertex>, Buffer<u32>) {
    println!("[HOTHAM_DRAW_GUI] Creating mesh buffers..");
    let vertices = (0..BUFFER_SIZE)
        .map(|_| Default::default())
        .collect::<Vec<_>>();
    let empty_index_buffer = [0; BUFFER_SIZE * 2];

    let vertex_buffer = Buffer::new(
        vulkan_context,
        &vertices,
        vk::BufferUsageFlags::VERTEX_BUFFER,
    )
    .expect("Unable to create font index buffer");

    let index_buffer = Buffer::new(
        vulkan_context,
        &empty_index_buffer,
        vk::BufferUsageFlags::INDEX_BUFFER,
    )
    .expect("Unable to create font index buffer");

    println!("[HOTHAM_DRAW_GUI] ..done!");

    (vertex_buffer, index_buffer)
}
