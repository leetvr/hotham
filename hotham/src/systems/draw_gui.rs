use legion::world::SubWorld;
use legion::{system, IntoQuery};

use crate::components::Panel;
use crate::resources::{GuiContext, HapticContext};
use crate::resources::{RenderContext, VulkanContext};

#[system]
#[write_component(Panel)]
pub fn draw_gui(
    world: &mut SubWorld,
    #[resource] vulkan_context: &VulkanContext,
    #[resource] swapchain_image_index: &usize,
    #[resource] render_context: &RenderContext,
    #[resource] gui_context: &mut GuiContext,
    #[resource] haptic_context: &mut HapticContext,
) {
    // Reset hovered_this_frame
    gui_context.hovered_this_frame = false;

    // Draw each panel
    let mut query = <&mut Panel>::query();
    query.for_each_mut(world, |panel| {
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
    });

    // Did we hover over a button in this frame? If so request haptic feedback.
    if !gui_context.hovered_last_frame && gui_context.hovered_this_frame {
        haptic_context.request_haptic_feedback(1.);
    }

    // Stash the value for the next frame.
    gui_context.hovered_last_frame = gui_context.hovered_this_frame;
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use ash::vk::{self, Handle};
    use egui::Pos2;
    use image::{jpeg::JpegEncoder, DynamicImage, RgbaImage};
    use legion::{IntoQuery, Resources, Schedule, World};
    use nalgebra::UnitQuaternion;
    use openxr::{Fovf, Quaternionf, Vector3f};
    use renderdoc::RenderDoc;

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
        let (mut world, mut resources, image, mut schedule) = setup(resolution.clone());

        let mut renderdoc: RenderDoc<renderdoc::V141> = RenderDoc::new().unwrap();

        // Begin. Use renderdoc in headless mode for debugging.
        renderdoc.start_frame_capture(std::ptr::null(), std::ptr::null());
        schedule.execute(&mut world, &mut resources);

        // Assert that haptic feedback has been requested.
        assert_eq!(get_haptic_amplitude(&mut resources), 1.0);

        // Assert the button WAS NOT clicked this frame
        assert!(!button_was_clicked(&mut world));

        // Release the trigger slightly
        change_panel_trigger_value(&mut world);
        schedule.execute(&mut world, &mut resources);

        // Assert that NO haptic feedback has been requested.
        assert_eq!(get_haptic_amplitude(&mut resources), 0.);

        // Assert the button WAS clicked this frame
        assert!(button_was_clicked(&mut world));

        // Move the cursor off the panel and release the trigger entirely
        move_cursor_off_panel(&mut world);
        schedule.execute(&mut world, &mut resources);

        // Assert the button WAS NOT clicked this frame
        assert!(!button_was_clicked(&mut world));

        // Assert that NO haptic feedback has been requested.
        assert_eq!(get_haptic_amplitude(&mut resources), 0.);

        renderdoc.end_frame_capture(std::ptr::null(), std::ptr::null());

        // Get the image off the GPU
        let vulkan_context = resources.get::<VulkanContext>().unwrap();
        write_image_to_disk(&vulkan_context, image, resolution);

        if !renderdoc.is_target_control_connected() {
            let _ = Command::new("explorer.exe")
                .args(["..\\test_assets\\render_gui.jpg"])
                .output()
                .unwrap();
        }
    }

    fn button_was_clicked(world: &mut World) -> bool {
        let mut query = <&mut Panel>::query();
        let panel = query.iter_mut(world).next().unwrap();
        return panel.buttons[0].clicked_this_frame;
    }

    fn get_haptic_amplitude(resources: &mut Resources) -> f32 {
        let haptic_context = resources.get::<HapticContext>().unwrap();
        return haptic_context.amplitude_this_frame;
    }

    fn change_panel_trigger_value(world: &mut World) {
        let mut query = <&mut Panel>::query();
        let panel = query.iter_mut(world).next().unwrap();
        panel.input = Some(PanelInput {
            cursor_location: Pos2::new(0.5 * (800. / SCALE_FACTOR), 0.05 * (800. / SCALE_FACTOR)),
            trigger_value: 0.2,
        });
    }

    fn move_cursor_off_panel(world: &mut World) {
        let mut query = <&mut Panel>::query();
        let panel = query.iter_mut(world).next().unwrap();
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

    pub fn setup(resolution: vk::Extent2D) -> (World, Resources, Image, Schedule) {
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
        world.push(panel_components);

        let haptic_context = HapticContext::default();

        let mut resources = Resources::default();
        resources.insert(vulkan_context);
        resources.insert(render_context);
        resources.insert(gui_context);
        resources.insert(0 as usize);
        resources.insert(haptic_context);

        let schedule = build_schedule();

        (world, resources, image, schedule)
    }

    fn build_schedule() -> Schedule {
        Schedule::builder()
            .add_thread_local_fn(|_, resources| {
                // let rotation: mint::Quaternion<f32> =
                //     UnitQuaternion::from_euler_angles(0., 45_f32.to_radians(), 0.).into();
                // let rotation: mint::Quaternion<f32> = UnitQuaternion::from_euler_angles(
                //     -10_f32.to_radians(),
                //     10_f32.to_radians(),
                //     0.,
                // )
                // .into();
                let rotation: mint::Quaternion<f32> =
                    UnitQuaternion::from_euler_angles(0., 0., 0.).into();
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
                let vulkan_context = resources.get::<VulkanContext>().unwrap();
                let mut render_context = resources.get_mut::<RenderContext>().unwrap();

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
            })
            .add_thread_local_fn(|_, resources| {
                // Reset the haptic context each frame - do this instead of having to create an OpenXR context etc.
                let mut haptic_context = resources.get_mut::<HapticContext>().unwrap();
                haptic_context.amplitude_this_frame = 0.;
            })
            .add_system(draw_gui_system())
            .add_thread_local_fn(|_, resources| {
                let vulkan_context = resources.get::<VulkanContext>().unwrap();
                let render_context = resources.get_mut::<RenderContext>().unwrap();
                render_context.begin_pbr_render_pass(&vulkan_context, 0);
            })
            .add_system(update_transform_matrix_system())
            .add_system(update_parent_transform_matrix_system())
            .add_system(rendering_system())
            .add_thread_local_fn(|_, resources| {
                let vulkan_context = resources.get::<VulkanContext>().unwrap();
                let mut render_context = resources.get_mut::<RenderContext>().unwrap();
                render_context.end_pbr_render_pass(&vulkan_context, 0);
            })
            .add_thread_local_fn(|_, resources| {
                let vulkan_context = resources.get::<VulkanContext>().unwrap();
                let mut render_context = resources.get_mut::<RenderContext>().unwrap();
                render_context.end_frame(&vulkan_context, 0);
            })
            .build()
    }
}
