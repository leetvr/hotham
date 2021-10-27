mod components;
mod systems;

use std::collections::HashMap;

use hotham::components::hand::Handedness;
use hotham::components::Transform;
use hotham::gltf_loader::add_model_to_world;
use hotham::legion::{Resources, Schedule, World};
use hotham::resources::{PhysicsContext, RenderContext, XrContext};
use hotham::schedule_functions::{begin_frame, end_frame, physics_step};
use hotham::systems::rendering::rendering_system;
use hotham::systems::{
    collision_system, update_parent_transform_matrix_system, update_rigid_body_transforms_system,
    update_transform_matrix_system,
};
use hotham::{gltf_loader, App, HothamResult};
use hotham_debug_server::DebugServer as DebugServerT;

use legion::EntityStore;
use nalgebra::{vector, Quaternion};
use serde::{Deserialize, Serialize};
use systems::sabers::{add_saber_physics, sabers_system};

use crate::components::Saber;

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
pub fn main() {
    println!("[BEAT_SABER_EXAMPLE] MAIN!");
    real_main().unwrap();
}

type EditableData = DebugInfo;
type NonEditableData = ();
type DebugServer = DebugServerT<EditableData, NonEditableData>;
type Models = HashMap<String, World>;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct DebugInfo {
    pub environment: Transform,
    pub ramp: Transform,
}

pub fn real_main() -> HothamResult<()> {
    let (xr_context, vulkan_context) = XrContext::new()?;
    let render_context = RenderContext::new(&vulkan_context, &xr_context)?;
    let mut physics_context = PhysicsContext::default();
    let mut world = World::default();
    let glb_bufs: Vec<&[u8]> = vec![include_bytes!("../assets/beat_saber.glb")];
    let models: Models = gltf_loader::load_models_from_glb(
        &glb_bufs,
        &vulkan_context,
        &render_context.descriptor_set_layouts,
    )?;

    // Add Environment
    let environment = add_model_to_world("Environment", &models, &mut world, None).unwrap();
    let ramp = add_model_to_world("Ramp", &models, &mut world, None).unwrap();

    // Add cubes
    // add_model_to_world("Red Cube", &models, &mut world, None).expect("Unable to add Red Cube");
    // add_model_to_world("Blue Cube", &models, &mut world, None).expect("Unable to add Blue Cube");

    // Add Red Saber
    let red_saber = add_model_to_world("Red Saber", &models, &mut world, None).unwrap();
    {
        add_saber_physics(&mut world, &mut physics_context, red_saber);
        let mut saber_entry = world.entry(red_saber).unwrap();
        saber_entry.add_component(Saber {
            handedness: Handedness::Left,
        });
    }

    // Add Blue Saber
    let blue_saber = add_model_to_world("Blue Saber", &models, &mut world, None).unwrap();
    {
        add_saber_physics(&mut world, &mut physics_context, blue_saber);
        let mut saber_entry = world.entry(blue_saber).unwrap();
        saber_entry.add_component(Saber {
            handedness: Handedness::Right,
        });
    }

    let debug_server = DebugServer::new();

    let mut resources = Resources::default();
    resources.insert(xr_context);
    resources.insert(vulkan_context);
    resources.insert(physics_context);
    resources.insert(debug_server);
    resources.insert(render_context);
    resources.insert(models);
    resources.insert(0 as usize);

    let schedule = Schedule::builder()
        .add_thread_local_fn(begin_frame)
        .add_thread_local_fn(move |world, r| {
            let mut debug_server = r.get_mut::<DebugServer>().unwrap();
            if let Some(updated) = debug_server.sync(&()) {
                // let mut entity = world.entry_mut(environment).unwrap();
                // *entity.get_component_mut::<Transform>().unwrap() = updated.environment;

                // let mut entity = world.entry_mut(ramp).unwrap();
                // *entity.get_component_mut::<Transform>().unwrap() = updated.ramp;
                // render_context
                //     .scene_params_buffer
                //     .update(&vulkan_context, &[updated])
                //     .expect("Unable to update data");
            };
        })
        .add_system(collision_system())
        .add_thread_local_fn(physics_step)
        .add_system(sabers_system())
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
