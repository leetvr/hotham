use hotham::{
    components::ui_panel::add_ui_panel_to_world,
    hecs::World,
    resources::{vulkan_context::VulkanContext, GuiContext, PhysicsContext, RenderContext},
    schedule_functions::{begin_frame, end_frame},
    systems::{
        draw_gui_system, rendering::rendering_system, update_transform_matrix_system, Queries,
    },
    vk, xr, Engine, HothamResult,
};

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
pub fn main() {
    println!("[HOTHAM_MULTIPLE_GUIS] MAIN!");
    real_main().expect("Error running app!");
    println!("[HOTHAM_MULTIPLE_GUIS] FINISHED! Goodbye!");
}

pub fn real_main() -> HothamResult<()> {
    let mut engine = Engine::new();
    let mut world = init(&mut engine)?;
    let mut queries = Default::default();

    while let Ok(xr_state) = engine.update() {
        tick(xr_state, &mut engine, &mut world, &mut queries);
    }

    Ok(())
}

pub fn add_gui(
    text: &str,
    y_pos: f32,
    vulkan_context: &mut VulkanContext,
    render_context: &mut RenderContext,
    gui_context: &GuiContext,
    physics_context: &mut PhysicsContext,
    world: &mut World,
) {
    add_ui_panel_to_world(
        text,
        vk::Extent2D {
            width: 1000,
            height: 100,
        },
        [1.0, 0.1].into(),
        [0., y_pos, -1.].into(),
        vec![],
        vulkan_context,
        render_context,
        gui_context,
        physics_context,
        world,
    );
}

fn init(engine: &mut Engine) -> Result<World, hotham::HothamError> {
    let render_context = &mut engine.render_context;
    let vulkan_context = &mut engine.vulkan_context;
    let gui_context = &mut engine.gui_context;
    let physics_context = &mut engine.physics_context;
    let mut world = World::default();

    add_gui(
        "hello",
        1.2,
        vulkan_context,
        render_context,
        gui_context,
        physics_context,
        &mut world,
    );
    add_gui(
        "stonks 📈",
        2.0,
        vulkan_context,
        render_context,
        gui_context,
        physics_context,
        &mut world,
    );

    Ok(world)
}

fn tick(
    xr_state: (xr::SessionState, xr::SessionState),
    engine: &mut Engine,
    world: &mut World,
    queries: &mut Queries,
) {
    let current_state = xr_state.1;
    // If we're not in a session, don't run the frame loop.
    match xr_state.1 {
        xr::SessionState::IDLE | xr::SessionState::EXITING | xr::SessionState::STOPPING => return,
        _ => {}
    }

    let xr_context = &mut engine.xr_context;
    let vulkan_context = &engine.vulkan_context;
    let render_context = &mut engine.render_context;
    let haptic_context = &mut engine.haptic_context;
    let gui_context = &mut engine.gui_context;

    let (_should_render, swapchain_image_index) = begin_frame(xr_context, render_context);

    if current_state == xr::SessionState::FOCUSED {
        update_transform_matrix_system(&mut queries.update_transform_matrix_query, world);
    }

    if current_state == xr::SessionState::FOCUSED || current_state == xr::SessionState::VISIBLE {
        render_context.begin_frame(vulkan_context, swapchain_image_index);
        draw_gui_system(
            &mut queries.draw_gui_query,
            world,
            vulkan_context,
            &swapchain_image_index,
            render_context,
            gui_context,
            haptic_context,
        );

        rendering_system(
            &mut queries.rendering_query,
            world,
            vulkan_context,
            swapchain_image_index,
            render_context,
        );
        render_context.end_frame(vulkan_context, swapchain_image_index);
    }

    end_frame(xr_context);
}
