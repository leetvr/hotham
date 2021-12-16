use hotham::gltf_loader::add_model_to_world;
use hotham::legion::{Resources, Schedule, World};
use hotham::rapier3d::na::vector;
use hotham::rapier3d::prelude::{ColliderBuilder, RigidBodyBuilder};
use hotham::resources::{PhysicsContext, RenderContext, XrContext};
use hotham::schedule_functions::{begin_frame, end_frame, physics_step, sync_debug_server};
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
    let debug_server: DebugServer = DebugServer::new();

    let blue_cube = add_model_to_world("Blue Cube", &models, &mut world, None)
        .expect("Unable to add Blue Cube");
    let red_cube =
        add_model_to_world("Red Cube", &models, &mut world, None).expect("Unable to add Red Cube");
    let blue_saber = add_model_to_world("Blue Saber", &models, &mut world, None)
        .expect("Unable to add Blue Saber");
    add_model_to_world("Red Saber", &models, &mut world, None).expect("Unable to add Red Saber");
    add_model_to_world("Environment", &models, &mut world, None)
        .expect("Unable to add Environment");
    add_model_to_world("Ramp", &models, &mut world, None).expect("Unable to add Ramp");

    // Add test physics objects
    let rigid_body = RigidBodyBuilder::new_dynamic()
        .translation(vector![0., 5., 0.])
        .build();
    let collider = ColliderBuilder::cylinder(1.0, 0.2).build();
    let (rigid_body, collider) =
        physics_context.add_rigid_body_and_collider(blue_saber, rigid_body, collider);
    {
        let mut entry = world.entry(blue_saber).unwrap();
        entry.add_component(rigid_body);
        entry.add_component(collider);
    }

    let rigid_body = RigidBodyBuilder::new_dynamic().build();
    let collider = ColliderBuilder::cuboid(1.0, 1.0, 1.0)
        .translation(vector![0., 1.0, 0.])
        .build();
    let (rigid_body, collider) =
        physics_context.add_rigid_body_and_collider(blue_cube, rigid_body, collider);
    {
        let mut entry = world.entry(blue_cube).unwrap();
        entry.add_component(rigid_body);
        entry.add_component(collider);
    }

    let rigid_body = RigidBodyBuilder::new_dynamic().build();
    let collider = ColliderBuilder::cuboid(1.0, 1.0, 1.0)
        .translation(vector![0., 1., 0.])
        .build();
    let (rigid_body, collider) =
        physics_context.add_rigid_body_and_collider(red_cube, rigid_body, collider);
    {
        let mut entry = world.entry(red_cube).unwrap();
        entry.add_component(rigid_body);
        entry.add_component(collider);
    }

    let mut resources = Resources::default();
    resources.insert(xr_context);
    resources.insert(vulkan_context);
    resources.insert(render_context);
    resources.insert(physics_context);
    resources.insert(0 as usize);
    resources.insert(debug_server);

    let schedule = Schedule::builder()
        .add_thread_local_fn(begin_frame)
        .add_system(collision_system())
        .add_thread_local_fn(physics_step)
        .add_system(update_rigid_body_transforms_system())
        .add_system(update_transform_matrix_system())
        .add_system(update_parent_transform_matrix_system())
        .add_system(rendering_system())
        .add_thread_local_fn(sync_debug_server)
        .add_thread_local_fn(end_frame)
        .build();

    let mut app = App::new(world, resources, schedule)?;
    app.run()?;
    Ok(())
}
