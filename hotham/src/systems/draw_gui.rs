use crate::{
    components::{hand::Handedness, Panel},
    resources::{GuiContext, HapticContext, RenderContext, VulkanContext},
};
use hecs::{PreparedQuery, World};
static GUI_HAPTIC_AMPLITUDE: f32 = 0.5;

pub fn draw_gui_system(
    query: &mut PreparedQuery<&mut Panel>,
    world: &mut World,
    vulkan_context: &VulkanContext,
    swapchain_image_index: &usize,
    render_context: &RenderContext,
    gui_context: &mut GuiContext,
    haptic_context: &mut HapticContext,
) {
    // Reset hovered_this_frame
    gui_context.hovered_this_frame = false;

    // Draw each panel
    for (_, panel) in query.query_mut(world) {
        // Reset the button state
        for button in &mut panel.buttons {
            button.clicked_this_frame = false;
        }

        gui_context.paint_gui(
            &vulkan_context,
            &render_context,
            *swapchain_image_index,
            panel,
        );
    }

    // Did we hover over a button in this frame? If so request haptic feedback.
    if !gui_context.hovered_last_frame && gui_context.hovered_this_frame {
        // TODO - We should really have two pointer hands..
        haptic_context.request_haptic_feedback(GUI_HAPTIC_AMPLITUDE, Handedness::Right);
    }

    // Stash the value for the next frame.
    gui_context.hovered_last_frame = gui_context.hovered_this_frame;
}

#[cfg(target_os = "windows")]
#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use ash::vk::{self, Handle};
    use egui::Pos2;
    use image::{jpeg::JpegEncoder, DynamicImage, RgbaImage};
    use nalgebra::UnitQuaternion;
    use openxr::{Fovf, Quaternionf, Vector3f};
    use std::process::Command;

    use crate::{
        buffer::Buffer,
        components::{
            panel::{add_panel_to_world, PanelButton, PanelInput},
            Panel,
        },
        gltf_loader,
        image::Image,
        resources::{
            gui_context::SCALE_FACTOR, GuiContext, HapticContext, PhysicsContext, RenderContext,
            VulkanContext,
        },
        scene_data::SceneParams,
        swapchain::Swapchain,
        systems::{
            rendering_system, update_parent_transform_matrix_system, update_transform_matrix_system,
        },
        util::get_from_device_memory,
        COLOR_FORMAT,
    };

    use super::draw_gui_system;

    #[test]
    pub fn test_draw_gui() {
        let resolution = vk::Extent2D {
            height: 800,
            width: 800,
        };
        let (
            mut world,
            image,
            vulkan_context,
            mut render_context,
            mut haptic_context,
            mut gui_context,
        ) = setup(resolution.clone());

        // Begin. Use renderdoc in headless mode for debugging.
        let mut renderdoc = begin_renderdoc();

        let mut query = Default::default();
        schedule(
            &mut query,
            &mut world,
            &mut gui_context,
            &mut haptic_context,
            &mut render_context,
            &vulkan_context,
        );

        // Assert that haptic feedback has been requested.
        assert_eq!(
            haptic_context.right_hand_amplitude_this_frame,
            GUI_HAPTIC_AMPLITUDE
        );

        // Assert the button WAS NOT clicked this frame
        assert!(!button_was_clicked(&mut world));

        // Release the trigger slightly
        release_trigger(&mut world, &mut query);
        schedule(
            &mut query,
            &mut world,
            &mut gui_context,
            &mut haptic_context,
            &mut render_context,
            &vulkan_context,
        );

        // Assert that NO haptic feedback has been requested.
        assert_eq!(haptic_context.right_hand_amplitude_this_frame, 0.);

        // Assert the button WAS clicked this frame
        assert!(button_was_clicked(&mut world));

        // Move the cursor off the panel and release the trigger entirely
        move_cursor_off_panel(&mut world, &mut query);
        schedule(
            &mut query,
            &mut world,
            &mut gui_context,
            &mut haptic_context,
            &mut render_context,
            &vulkan_context,
        );

        // Assert the button WAS NOT clicked this frame
        assert!(!button_was_clicked(&mut world));

        // Assert that NO haptic feedback has been requested.
        assert_eq!(haptic_context.right_hand_amplitude_this_frame, 0.);

        end_renderdoc(&mut renderdoc);

        // Get the image off the GPU
        write_image_to_disk(&vulkan_context, image, resolution);

        open_file(&mut renderdoc);
    }

    fn schedule(
        query: &mut PreparedQuery<&mut Panel>,
        world: &mut World,
        gui_context: &mut GuiContext,
        haptic_context: &mut HapticContext,
        render_context: &mut RenderContext,
        vulkan_context: &VulkanContext,
    ) {
        println!("[DRAW_GUI_TEST] Running schedule..");
        begin_frame(render_context, vulkan_context);

        // Reset the haptic context each frame - do this instead of having to create an OpenXR context etc.
        haptic_context.right_hand_amplitude_this_frame = 0.;

        // Draw the GUI
        draw_gui_system(
            query,
            world,
            vulkan_context,
            &0,
            render_context,
            gui_context,
            haptic_context,
        );

        // Begin the PBR Render Pass
        render_context.begin_pbr_render_pass(vulkan_context, 0);

        // Update transforms, etc.
        update_transform_matrix_system(&mut Default::default(), world);

        // Update parent transform matrix
        update_parent_transform_matrix_system(
            &mut Default::default(),
            &mut Default::default(),
            world,
        );

        // Render
        rendering_system(
            &mut Default::default(),
            world,
            vulkan_context,
            0,
            render_context,
        );

        // End PBR render
        render_context.end_pbr_render_pass(vulkan_context, 0);
        render_context.end_frame(vulkan_context, 0);
    }

    fn begin_frame(render_context: &mut RenderContext, vulkan_context: &VulkanContext) {
        let rotation: mint::Quaternion<f32> = UnitQuaternion::from_euler_angles(0., 0., 0.).into();
        let position = Vector3f {
            x: -1.0,
            y: 0.0,
            z: 1.0,
        };

        let view = openxr::View {
            pose: openxr::Posef {
                orientation: Quaternionf::from(rotation),
                position,
            },
            fov: Fovf {
                angle_up: 45.0_f32.to_radians(),
                angle_down: -45.0_f32.to_radians(),
                angle_left: -45.0_f32.to_radians(),
                angle_right: 45.0_f32.to_radians(),
            },
        };
        let views = vec![view.clone(), view];

        render_context.begin_frame(&vulkan_context, 0);

        render_context
            .update_scene_data(&views, &vulkan_context)
            .unwrap();
        render_context
            .scene_params_buffer
            .update(
                &vulkan_context,
                &[SceneParams {
                    // debug_view_inputs: 1.,
                    ..Default::default()
                }],
            )
            .unwrap();
    }

    fn button_was_clicked(world: &mut World) -> bool {
        let panel = world
            .query_mut::<&mut Panel>()
            .into_iter()
            .next()
            .unwrap()
            .1;
        return panel.buttons[0].clicked_this_frame;
    }

    fn release_trigger(world: &mut World, query: &mut PreparedQuery<&mut Panel>) {
        let panel = query.query_mut(world).into_iter().next().unwrap().1;
        panel.input = Some(PanelInput {
            cursor_location: Pos2::new(0.5 * (800. / SCALE_FACTOR), 0.15 * (800. / SCALE_FACTOR)),
            trigger_value: 0.2,
        });
    }

    fn move_cursor_off_panel(world: &mut World, query: &mut PreparedQuery<&mut Panel>) {
        let panel = query.query_mut(world).into_iter().next().unwrap().1;
        panel.input = Some(PanelInput {
            cursor_location: Pos2::new(0., 0.),
            trigger_value: 0.0,
        });
    }

    fn write_image_to_disk(vulkan_context: &VulkanContext, image: Image, resolution: vk::Extent2D) {
        let size = (resolution.height * resolution.width * 4) as usize;
        let image_data = vec![0; size];
        let buffer = Buffer::new(
            &vulkan_context,
            &image_data,
            vk::BufferUsageFlags::TRANSFER_DST,
        )
        .unwrap();
        vulkan_context.transition_image_layout(
            image.handle,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            1,
            1,
        );
        vulkan_context.copy_image_to_buffer(
            &image,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            buffer.handle,
        );
        let image_bytes = unsafe { get_from_device_memory(&vulkan_context, &buffer) }.to_vec();
        let image_from_vulkan = DynamicImage::ImageRgba8(
            RgbaImage::from_raw(resolution.width, resolution.height, image_bytes).unwrap(),
        );
        let output_path = "../test_assets/render_gui.jpg";
        {
            let output_path = std::path::Path::new(&output_path);
            let mut file = std::fs::File::create(output_path).unwrap();
            let mut jpeg_encoder = JpegEncoder::new(&mut file);
            jpeg_encoder.encode_image(&image_from_vulkan).unwrap();
        }
    }

    pub fn setup(
        resolution: vk::Extent2D,
    ) -> (
        World,
        Image,
        VulkanContext,
        RenderContext,
        HapticContext,
        GuiContext,
    ) {
        let vulkan_context = VulkanContext::testing().unwrap();
        let mut physics_context = PhysicsContext::default();
        // Create an image with vulkan_context
        let image = vulkan_context
            .create_image(
                COLOR_FORMAT,
                &resolution,
                vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC,
                2,
                1,
            )
            .unwrap();
        vulkan_context
            .set_debug_name(vk::ObjectType::IMAGE, image.handle.as_raw(), "Screenshot")
            .unwrap();

        let swapchain = Swapchain {
            images: vec![image.handle],
            resolution,
        };

        let render_context =
            RenderContext::new_from_swapchain(&vulkan_context, &swapchain).unwrap();
        let gui_context = GuiContext::new(&vulkan_context);

        let gltf_data: Vec<&[u8]> = vec![include_bytes!(
            "../../../test_assets/ferris-the-crab/source/ferris.glb"
        )];
        let mut models = gltf_loader::load_models_from_glb(
            &gltf_data,
            &vulkan_context,
            &render_context.descriptor_set_layouts,
        )
        .unwrap();
        let (_, mut world) = models.drain().next().unwrap();

        let panel = add_panel_to_world(
            "Hello..",
            800,
            800,
            vec![
                PanelButton::new("Click me!"),
                PanelButton::new("Don't click me!"),
            ],
            [-1.0, 0., 0.].into(),
            &vulkan_context,
            &render_context,
            &gui_context,
            &mut physics_context,
            &mut world,
        );
        world.get_mut::<Panel>(panel).unwrap().input = Some(PanelInput {
            cursor_location: Pos2::new(0.5 * (800. / SCALE_FACTOR), 0.15 * (800. / SCALE_FACTOR)),
            trigger_value: 1.,
        });

        let haptic_context = HapticContext::default();

        (
            world,
            image,
            vulkan_context,
            render_context,
            haptic_context,
            gui_context,
        )
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    use renderdoc::RenderDoc;

    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    fn begin_renderdoc() -> Result<RenderDoc<renderdoc::V141>> {
        let mut renderdoc = RenderDoc::<renderdoc::V141>::new()?;
        renderdoc.start_frame_capture(std::ptr::null(), std::ptr::null());
        Ok(renderdoc)
    }

    #[cfg(target_os = "windows")]
    fn open_file(renderdoc: &mut Result<RenderDoc<renderdoc::V141>>) {
        if !renderdoc
            .as_mut()
            .map(|r| r.is_target_control_connected())
            .unwrap_or(false)
        {
            let _ = Command::new("explorer.exe")
                .args(["..\\test_assets\\render_gui.jpg"])
                .output()
                .unwrap();
        }
    }

    #[cfg(target_os = "macos")]
    fn open_file(_: &mut ()) {
        let _ = Command::new("open")
            .args(["../test_assets/render_gui.jpg"])
            .output()
            .unwrap();
    }

    // TODO: Support opening files on Linux
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    fn open_file(_: &mut Result<RenderDoc<renderdoc::V141>>) {}

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    fn begin_renderdoc() {}

    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    fn end_renderdoc(renderdoc: &mut Result<RenderDoc<renderdoc::V141>>) {
        let _ = renderdoc
            .as_mut()
            .map(|r| r.end_frame_capture(std::ptr::null(), std::ptr::null()));
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    fn end_renderdoc(_: &mut ()) {}
}
