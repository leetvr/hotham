mod components;
mod systems;
use components::Saber;
use hotham::{
    components::{hand::Handedness, Transform, TransformMatrix},
    gltf_loader::{self, add_model_to_world},
    legion::{Resources, Schedule, World},
    resources::{PhysicsContext, RenderContext, XrContext},
    schedule_functions::{begin_frame, end_frame, physics_step},
    systems::{
        collision_system, rendering::rendering_system, update_parent_transform_matrix_system,
        update_rigid_body_transforms_system, update_transform_matrix_system,
    },
    App, HothamResult,
};
use systems::sabers_system;

use crate::systems::sabers::add_saber_physics;

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
    add_model_to_world("Red Cube", &models, &mut world, None).expect("Unable to add Red Cube");
    add_model_to_world("Blue Cube", &models, &mut world, None).expect("Unable to add Blue Cube");

    // Add Red Saber
    let red_saber = add_model_to_world("Red Saber", &models, &mut world, None)
        .expect("Unable to add Red Saber");
    add_saber_physics(&mut world, &mut physics_context, red_saber);
    let mut red_saber_entry = world.entry(red_saber).unwrap();
    red_saber_entry.add_component(Saber {
        handedness: Handedness::Left,
    });
    red_saber_entry.add_component(Transform::default());
    red_saber_entry.add_component(TransformMatrix::default());

    // Add Blue Saber
    let blue_saber = add_model_to_world("Blue Saber", &models, &mut world, None)
        .expect("Unable to add Blue Saber");
    add_saber_physics(&mut world, &mut physics_context, blue_saber);
    let mut blue_saber_entry = world.entry(blue_saber).unwrap();
    blue_saber_entry.add_component(Saber {
        handedness: Handedness::Left,
    });
    blue_saber_entry.add_component(Transform::default());
    blue_saber_entry.add_component(TransformMatrix::default());

    // Add Environment
    add_model_to_world("Environment", &models, &mut world, None)
        .expect("Unable to add Environment");

    // Add Ramp
    add_model_to_world("Ramp", &models, &mut world, None).expect("Unable to add Ramp");

    let mut resources = Resources::default();
    resources.insert(xr_context);
    resources.insert(vulkan_context);
    resources.insert(render_context);
    resources.insert(physics_context);
    resources.insert(0 as usize);

    let schedule = Schedule::builder()
        .add_thread_local_fn(begin_frame)
        .add_system(sabers_system())
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
