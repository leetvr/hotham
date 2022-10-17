use crate::{
    components::{hand::Handedness, Panel, UIPanel},
    contexts::{GuiContext, HapticContext, RenderContext, VulkanContext},
    Engine,
};
use hecs::World;
static GUI_HAPTIC_AMPLITUDE: f32 = 0.5;

/// GUI system
/// Walks through each panel in the World and
/// - draws the panel to a texture
/// - updates any input state
pub fn draw_gui_system(engine: &mut Engine) {
    let world = &mut engine.world;
    let vulkan_context = &mut engine.vulkan_context;
    let render_context = &mut engine.render_context;
    let gui_context = &mut engine.gui_context;
    let haptic_context = &mut engine.haptic_context;

    draw_gui_system_inner(
        world,
        vulkan_context,
        render_context,
        gui_context,
        haptic_context,
    );
}

fn draw_gui_system_inner(
    world: &mut World,
    vulkan_context: &VulkanContext,
    render_context: &mut RenderContext,
    gui_context: &mut GuiContext,
    haptic_context: &mut HapticContext,
) {
    let mut new_hover = false;

    // Draw each panel
    for (_, (panel, ui_panel)) in world.query_mut::<(&mut Panel, &mut UIPanel)>() {
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
    use super::*;
    use ash::vk::{self, Handle};
    use egui::Pos2;
    use openxr::{Fovf, Quaternionf, Vector3f};

    use crate::{
        asset_importer,
        components::{
            panel::PanelInput,
            ui_panel::{add_ui_panel_to_world, UIPanelButton},
            UIPanel,
        },
        contexts::{GuiContext, HapticContext, RenderContext, VulkanContext},
        rendering::{image::Image, swapchain::SwapchainInfo},
        systems::{
            rendering::rendering_system_inner,
            update_global_transform::update_global_transform_system_inner,
            update_global_transform_with_parent::update_global_transform_with_parent_system_inner,
        },
        util::save_image_to_disk,
        COLOR_FORMAT,
    };

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
        // let mut renderdoc = begin_renderdoc();

        draw(
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
        release_trigger(&mut world);
        draw(
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
        move_cursor_off_panel(&mut world);
        draw(
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

        // if let Ok(renderdoc) = renderdoc.as_mut() {
        //     end_renderdoc(renderdoc);
        // }

        // Get the image off the GPU
        unsafe { save_image_to_disk(&vulkan_context, image, "draw_gui").unwrap() };
    }

    fn draw(
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
        update_global_transform_system_inner(world);

        // Update parent transform matrix
        update_global_transform_with_parent_system_inner(world);

        // Render
        render_context.begin_frame(&vulkan_context);

        // Draw the GUI
        println!("[DRAW_GUI_TEST] draw_gui_system");
        draw_gui_system_inner(
            world,
            vulkan_context,
            render_context,
            gui_context,
            haptic_context,
        );

        let views = get_views();
        println!("[DRAW_GUI_TEST] rendering_system");
        rendering_system_inner(world, vulkan_context, render_context, &views, 0);
        render_context.end_frame(vulkan_context);
    }

    fn get_views() -> Vec<openxr::View> {
        let rotation: mint::Quaternion<f32> = Quaternionf::IDENTITY.into();
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

    fn release_trigger(world: &mut World) {
        let (panel, _ui_panel) = world
            .query_mut::<(&mut Panel, &mut UIPanel)>()
            .into_iter()
            .next()
            .unwrap()
            .1;
        panel.input = Some(PanelInput {
            cursor_location: Pos2::new(0.5 * 800., 0.15 * 800.),
            trigger_value: 0.2,
        });
    }

    fn move_cursor_off_panel(world: &mut World) {
        let (panel, _ui_panel) = world
            .query_mut::<(&mut Panel, &mut UIPanel)>()
            .into_iter()
            .next()
            .unwrap()
            .1;
        panel.input = Some(PanelInput {
            cursor_location: Pos2::new(0., 0.),
            trigger_value: 0.0,
        });
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
            &mut world,
        );
        world.get::<&mut Panel>(panel).unwrap().input = Some(PanelInput {
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
}
