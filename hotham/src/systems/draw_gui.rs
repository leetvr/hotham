use crate::{
    components::Panel,
    resources::{GuiContext, HapticContext, RenderContext, VulkanContext},
};
use hecs::{PreparedQuery, World};

pub fn draw_gui_system(
    query: &PreparedQuery<&mut Panel>,
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
        haptic_context.request_haptic_feedback(1.);
    }

    // Stash the value for the next frame.
    gui_context.hovered_last_frame = gui_context.hovered_this_frame;
}

#[cfg(test)]
mod tests {
    use super::*;
    use ash::vk::{self, Handle};
    use egui::Pos2;
    use image::{jpeg::JpegEncoder, DynamicImage, RgbaImage};
    use nalgebra::UnitQuaternion;
    use openxr::{Fovf, Quaternionf, Vector3f};
    use renderdoc::RenderDoc;
    use std::process::Command;

    use crate::{
        buffer::Buffer,
        components::{
            panel::{create_panel, PanelButton, PanelInput},
            Panel,
        },
        gltf_loader,
        image::Image,
        resources::{
            gui_context::SCALE_FACTOR, GuiContext, HapticContext, RenderContext, VulkanContext,
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
        let (mut world, image, vulkan_context, render_context, mut haptic_context, mut gui_context) =
            setup(resolution.clone());

        let mut renderdoc: RenderDoc<renderdoc::V141> = RenderDoc::new().unwrap();

        // Begin. Use renderdoc in headless mode for debugging.
        renderdoc.start_frame_capture(std::ptr::null(), std::ptr::null());
        let query = PreparedQuery::<&mut Panel>::default();
        schedule(
            &query,
            &mut world,
            &mut gui_context,
            &mut haptic_context,
            &render_context,
            &vulkan_context,
        );

        // Assert that haptic feedback has been requested.
        assert_eq!(haptic_context.amplitude_this_frame, 0.1);

        // Assert the button WAS NOT clicked this frame
        assert!(!button_was_clicked(&mut world));

        // Release the trigger slightly
        change_panel_trigger_value(&mut world, &query);
        schedule(
            &query,
            &mut world,
            &mut gui_context,
            &mut haptic_context,
            &render_context,
            &vulkan_context,
        );

        // Assert that NO haptic feedback has been requested.
        assert_eq!(haptic_context.amplitude_this_frame, 0.);

        // Assert the button WAS clicked this frame
        assert!(button_was_clicked(&mut world));

        // Move the cursor off the panel and release the trigger entirely
        move_cursor_off_panel(&mut world, &query);
        schedule(
            &query,
            &mut world,
            &mut gui_context,
            &mut haptic_context,
            &render_context,
            &vulkan_context,
        );

        // Assert the button WAS NOT clicked this frame
        assert!(!button_was_clicked(&mut world));

        // Assert that NO haptic feedback has been requested.
        assert_eq!(haptic_context.amplitude_this_frame, 0.);

        renderdoc.end_frame_capture(std::ptr::null(), std::ptr::null());

        // Get the image off the GPU
        write_image_to_disk(&vulkan_context, image, resolution);

        if !renderdoc.is_target_control_connected() {
            let _ = Command::new("explorer.exe")
                .args(["..\\test_assets\\render_gui.jpg"])
                .output()
                .unwrap();
        }
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

    fn change_panel_trigger_value(world: &mut World, query: &PreparedQuery<&mut Panel>) {
        let panel = query.query_mut(world).into_iter().next().unwrap().1;
        panel.input = Some(PanelInput {
            cursor_location: Pos2::new(0.5 * (800. / SCALE_FACTOR), 0.05 * (800. / SCALE_FACTOR)),
            trigger_value: 0.2,
        });
    }

    fn move_cursor_off_panel(world: &mut World, query: &PreparedQuery<&mut Panel>) {
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

        let mut panel_components = create_panel(
            "Hello..",
            800,
            800,
            &vulkan_context,
            &render_context,
            &gui_context,
            vec![PanelButton::new("Click me!")],
        );
        panel_components.0.input = Some(PanelInput {
            cursor_location: Pos2::new(0.5 * (800. / SCALE_FACTOR), 0.05 * (800. / SCALE_FACTOR)),
            trigger_value: 1.,
        });
        panel_components.3.translation[0] = -1.0;
        world.spawn(panel_components);

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

    fn schedule(
        query: &PreparedQuery<&mut Panel>,
        world: &mut World,
        gui_context: &mut GuiContext,
        haptic_context: &mut HapticContext,
        render_context: &RenderContext,
        vulkan_context: &VulkanContext,
    ) -> () {
        begin_frame(render_context, vulkan_context);

        // Reset the haptic context each frame - do this instead of having to create an OpenXR context etc.
        haptic_context.amplitude_this_frame = 0.;

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
        update_transform_matrix_system();

        // Update parent transform matrix
        update_parent_transform_matrix_system();

        // Render
        rendering_system();

        // End PBR render
        render_context.end_pbr_render_pass(vulkan_context, 0);
        render_context.end_frame(vulkan_context, 0);
    }

    fn begin_frame(render_context: &RenderContext, vulkan_context: &VulkanContext) {
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

        render_context.begin_frame(&vulkan_context, 0);
    }
}
