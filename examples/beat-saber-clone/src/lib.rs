use hotham::gltf_loader::add_model_to_world;
use hotham::legion::{Resources, Schedule, World};
use hotham::resources::vulkan_context::VulkanContext;
use hotham::resources::{PhysicsContext, RenderContext, XrContext};
use hotham::scene_data::{SceneData, SceneParams};
use hotham::schedule_functions::{begin_frame, end_frame, physics_step};
use hotham::systems::rendering::rendering_system;
use hotham::systems::{
    collision_system, update_parent_transform_matrix_system, update_rigid_body_transforms_system,
    update_transform_matrix_system,
};
use hotham::{gltf_loader, App, HothamResult};
use hotham_debug_server::DebugServer;

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
pub fn main() {
    println!("[BEAT_SABER_EXAMPLE] MAIN!");
    real_main().unwrap();
}

pub fn real_main() -> HothamResult<()> {
    let (xr_context, vulkan_context) = XrContext::new()?;
    let render_context = RenderContext::new(&vulkan_context, &xr_context)?;
    let mut physics_context = PhysicsContext::default();
    let mut world = World::default();
    let glb_bufs: Vec<&[u8]> = vec![include_bytes!("../assets/beat_saber.glb")];
    let models = gltf_loader::load_models_from_glb(
        &glb_bufs,
        &vulkan_context,
        &render_context.descriptor_set_layouts,
    )?;
    // Add cubes
    // add_model_to_world("Red Cube", &models, &mut world, None).expect("Unable to add Red Cube");
    // add_model_to_world("Blue Cube", &models, &mut world, None).expect("Unable to add Blue Cube");

    // Add Red Saber
    let saber_offset = world.push((Transform::default(), TransformMatrix::default()));
    {
        add_saber_physics(&mut world, &mut physics_context, saber_offset);
        let mut saber_entry = world.entry(saber_offset).unwrap();
        saber_entry.add_component(Saber {
            handedness: Handedness::Left,
        });
    }
    let _red_saber = add_model_to_world("Red Saber", &models, &mut world, Some(saber_offset))
        .expect("Unable to add Red Saber");

    // Add Blue Saber
    // let blue_saber = add_model_to_world("Blue Saber", &models, &mut world, None)
    //     .expect("Unable to add Blue Saber");
    let schedule = Schedule::builder()
        .add_thread_local_fn(begin_frame)
        .add_thread_local_fn(|_, resources| {
            let vulkan_context = resources.get::<VulkanContext>().unwrap();
            let mut debug_server = resources
                .get_mut::<DebugServer<SceneParams, SceneData>>()
                .unwrap();
            if let Some(updated) = debug_server.sync(&render_context.scene_data) {
                render_context
                    .scene_params_buffer
                    .update(&vulkan_context, &[updated])
                    .expect("Unable to update data");
            };
        })
        .add_system(collision_system())
        .add_thread_local_fn(physics_step)
        .add_system(update_rigid_body_transforms_system())
        .add_system(update_transform_matrix_system())
        .add_system(update_parent_transform_matrix_system())
        .add_system(rendering_system())
        .add_thread_local_fn(end_frame)
        .build();

    let mut app = App::new(world, resources, schedule)?;
    app.run()?;
    Ok(())
}
