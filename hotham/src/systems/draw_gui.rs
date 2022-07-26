use crate::{
    components::{hand::Handedness, Panel, UIPanel},
    resources::{GuiContext, HapticContext, RenderContext, VulkanContext},
};
use hecs::{PreparedQuery, World};
static GUI_HAPTIC_AMPLITUDE: f32 = 0.5;

/// GUI system
/// Walks through each panel in the World and
/// - draws the panel to a texture
/// - updates any input state
pub fn draw_gui_system(
    query: &mut PreparedQuery<(&mut Panel, &mut UIPanel)>,
    world: &mut World,
    vulkan_context: &VulkanContext,
    render_context: &mut RenderContext,
    gui_context: &mut GuiContext,
    haptic_context: &mut HapticContext,
) {
    let mut new_hover = false;

    // Draw each panel
    for (_, (panel, ui_panel)) in query.query_mut(world) {
        // Reset the button state
        for button in &mut ui_panel.buttons {
            button.hovered_this_frame = false;
            button.clicked_this_frame = false;
        }

        gui_context.paint_gui(vulkan_context, render_context, ui_panel, panel);

        for button in &mut ui_panel.buttons {
            if !button.hovered_last_frame && button.hovered_this_frame {
                new_hover = true;
            }
            // Stash the value for the next frame.
            button.hovered_last_frame = button.hovered_this_frame;
        }
    }

    // Did we hover over a button in this frame? If so request haptic feedback.
    if new_hover {
        // TODO - We should really have two pointer hands..
        haptic_context.request_haptic_feedback(GUI_HAPTIC_AMPLITUDE, Handedness::Right);
    }
}

#[cfg(target_os = "windows")]
#[cfg(test)]
mod tests {
    #![allow(deprecated)]

    use super::*;
    use anyhow::Result;
    use ash::vk::{self, Handle};
    use egui::Pos2;
    use image::{codecs::jpeg::JpegEncoder, DynamicImage, RgbaImage};
    use nalgebra::UnitQuaternion;
    use openxr::{Fovf, Quaternionf, Vector3f};
    use std::process::Command;

    use crate::{
        asset_importer,
        components::{
            panel::PanelInput,
            ui_panel::{add_ui_panel_to_world, UIPanelButton},
            UIPanel,
        },
        rendering::{image::Image, legacy_buffer::Buffer, swapchain::SwapchainInfo},
        resources::{GuiContext, HapticContext, PhysicsContext, RenderContext, VulkanContext},
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
        draw(
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
        draw(
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
        draw(
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

    fn draw(
        query: &mut PreparedQuery<(&mut Panel, &mut UIPanel)>,
        world: &mut World,
        gui_context: &mut GuiContext,
        haptic_context: &mut HapticContext,
        render_context: &mut RenderContext,
        vulkan_context: &VulkanContext,
    ) {
        println!("[DRAW_GUI_TEST] Beginning frame..");

        // Reset the haptic context each frame - do this instead of having to create an OpenXR context etc.
        haptic_context.right_hand_amplitude_this_frame = 0.;

        // Update transforms, etc.
        update_transform_matrix_system(&mut Default::default(), world);

        // Update parent transform matrix
        update_parent_transform_matrix_system(
            &mut Default::default(),
            &mut Default::default(),
            world,
        );

        // Draw the GUI
        println!("[DRAW_GUI_TEST] draw_gui_system");
        draw_gui_system(
            query,
            world,
            vulkan_context,
            render_context,
            gui_context,
            haptic_context,
        );

        // Render
        println!("[DRAW_GUI_TEST] rendering_system");
        render_context.begin_frame(&vulkan_context);
        let views = get_views();
        rendering_system(
            &mut Default::default(),
            world,
            vulkan_context,
            render_context,
            &views,
            0,
        );
        render_context.end_frame(vulkan_context);
    }

    fn get_views() -> Vec<openxr::View> {
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
        vec![view.clone(), view]
    }

    fn button_was_clicked(world: &mut World) -> bool {
        let panel = world
            .query_mut::<&mut UIPanel>()
            .into_iter()
            .next()
            .unwrap()
            .1;
        return panel.buttons[0].clicked_this_frame;
    }

    fn release_trigger(world: &mut World, query: &mut PreparedQuery<(&mut Panel, &mut UIPanel)>) {
        let (panel, _ui_panel) = query.query_mut(world).into_iter().next().unwrap().1;
        panel.input = Some(PanelInput {
            cursor_location: Pos2::new(0.5 * 800., 0.15 * 800.),
            trigger_value: 0.2,
        });
    }

    fn move_cursor_off_panel(
        world: &mut World,
        query: &mut PreparedQuery<(&mut Panel, &mut UIPanel)>,
    ) {
        let (panel, _ui_panel) = query.query_mut(world).into_iter().next().unwrap().1;
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

        let swapchain = SwapchainInfo {
            images: vec![image.handle],
            resolution,
        };

        let mut render_context =
            RenderContext::new_from_swapchain_info(&vulkan_context, &swapchain).unwrap();
        let gui_context = GuiContext::new(&vulkan_context);

        let gltf_data: Vec<&[u8]> = vec![include_bytes!(
            "../../../test_assets/ferris-the-crab/source/ferris.glb"
        )];
        let mut models =
            asset_importer::load_models_from_glb(&gltf_data, &vulkan_context, &mut render_context)
                .unwrap();
        let (_, mut world) = models.drain().next().unwrap();

        let panel = add_ui_panel_to_world(
            "Hello..",
            vk::Extent2D {
                width: 800,
                height: 800,
            },
            [1., 1.].into(),
            [-1.0, 0., 0.].into(),
            vec![
                UIPanelButton::new("Click me!"),
                UIPanelButton::new("Don't click me!"),
            ],
            &vulkan_context,
            &mut render_context,
            &gui_context,
            &mut physics_context,
            &mut world,
        );
        world.get_mut::<Panel>(panel).unwrap().input = Some(PanelInput {
            cursor_location: Pos2::new(0.5 * 800., 0.15 * 800.),
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
