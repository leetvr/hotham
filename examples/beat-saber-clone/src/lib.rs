mod components;
pub mod resources;
mod systems;

use crate::{
    components::{Colour, Saber},
    resources::GameState,
    systems::{
        cube_spawner::{create_cubes, cube_spawner_system},
        game_system,
        sabers::{add_saber_physics, sabers_system},
        update_ui_system,
    },
};
use hotham_debug_server::DebugServer;
use rapier3d::prelude::{ColliderBuilder, InteractionGroups};
use std::collections::HashMap;

use hotham::{
    components::{
        hand::Handedness,
        panel::{create_panel, PanelButton},
        Collider, Pointer,
    },
    gltf_loader::{self, add_model_to_world},
    legion::{Resources, Schedule, World},
    resources::{
        physics_context::PANEL_COLLISION_GROUP, AudioContext, GuiContext, HapticContext,
        PhysicsContext, RenderContext, XrContext,
    },
    schedule_functions::{
        apply_haptic_feedback, begin_frame, begin_pbr_renderpass, end_frame, end_pbr_renderpass,
        physics_step, sync_debug_server,
    },
    systems::{
        collision_system, draw_gui_system, pointers_system, rendering_system,
        update_parent_transform_matrix_system, update_rigid_body_transforms_system,
        update_transform_matrix_system,
    },
    util::entity_to_u64,
    App, HothamResult,
};

use nalgebra::{vector, Matrix4, Vector3};
use serde::{Deserialize, Serialize};

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
pub fn main() {
    println!("[BEAT_SABER_EXAMPLE] MAIN!");
    real_main().expect("[BEAT_SABER_EXAMPLE] ERROR IN MAIN!");
}

#[cfg(target_os = "android")]
const SPAWN_RATE: usize = 100;

#[cfg(not(target_os = "android"))]
const SPAWN_RATE: usize = 1000;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct DebugInfo {
    position: Vector3<f32>,
    translation: Matrix4<f32>,
}

pub fn real_main() -> HothamResult<()> {
    let (xr_context, vulkan_context) = XrContext::new()?;
    let render_context = RenderContext::new(&vulkan_context, &xr_context)?;
    let gui_context = GuiContext::new(&vulkan_context);
    let mut physics_context = PhysicsContext::default();
    let mut world = World::default();
    let glb_bufs: Vec<&[u8]> = vec![include_bytes!("../assets/beat_saber.glb")];
    let models = gltf_loader::load_models_from_glb(
        &glb_bufs,
        &vulkan_context,
        &render_context.descriptor_set_layouts,
    )?;
    let haptic_context = HapticContext::default();
    let mut audio_context = AudioContext::default();

    // Add Environment
    add_environment_models(&models, &mut world, &vulkan_context, &render_context);

    // Add cubes
    create_cubes(
        50,
        &models,
        &mut world,
        &mut physics_context,
        &vulkan_context,
        &render_context.descriptor_set_layouts,
    );

    // Add Red Saber
    add_saber(
        Colour::Red,
        &models,
        &mut world,
        &vulkan_context,
        &render_context,
        &mut physics_context,
    );

    // Add Sound
    let mp3_bytes = include_bytes!("../../../test_assets/Quartet 14 - Clip.mp3").to_vec();
    let audio_source = audio_context.create_audio_source(mp3_bytes);
    world.push((audio_source,));

    // // Add Blue Saber
    // add_saber(
    //     Colour::Blue,
    //     &models,
    //     &mut world,
    //     &vulkan_context,
    //     &render_context,
    //     &mut physics_context,
    // );

    // Add pointer
    add_pointer(
        Colour::Blue,
        &models,
        &mut world,
        &vulkan_context,
        &render_context,
    );

    // Add a panel
    let mut panel_components = create_panel(
        "Not clicked!",
        800,
        800,
        &vulkan_context,
        &render_context,
        &gui_context,
        vec![PanelButton::new("Test")],
    );
    let t = &mut panel_components.3.translation;
    t[1] = 1.5;
    t[2] = -2.;

    let panel_entity = world.push(panel_components);
    let collider = ColliderBuilder::cuboid(0.5, 0.5, 0.)
        .sensor(true)
        .collision_groups(InteractionGroups::new(
            PANEL_COLLISION_GROUP,
            PANEL_COLLISION_GROUP,
        ))
        .translation(vector![0.0, 1.5, -2.])
        .user_data(entity_to_u64(panel_entity).into())
        .build();
    let handle = physics_context.colliders.insert(collider);
    let collider = Collider {
        collisions_this_frame: Vec::new(),
        handle,
    };
    let mut panel_entry = world.entry(panel_entity).unwrap();
    panel_entry.add_component(collider);

    let debug_server = DebugServer::new();

    let mut resources = Resources::default();
    resources.insert(xr_context);
    resources.insert(vulkan_context);
    resources.insert(gui_context);
    resources.insert(physics_context);
    resources.insert(debug_server);
    resources.insert(render_context);
    resources.insert(models);
    resources.insert(0 as usize);
    resources.insert(GameState::default());
    resources.insert(haptic_context);
    resources.insert(audio_context);

    let schedule = Schedule::builder()
        .add_thread_local_fn(begin_frame)
        .add_system(sabers_system())
        .add_system(pointers_system())
        .add_thread_local_fn(physics_step)
        .add_system(collision_system())
        .add_system(cube_spawner_system(SPAWN_RATE))
        .add_system(update_rigid_body_transforms_system())
        .add_system(update_transform_matrix_system())
        .add_system(update_parent_transform_matrix_system())
        .add_system(game_system())
        .add_system(update_ui_system())
        .add_system(draw_gui_system())
        .add_thread_local_fn(begin_pbr_renderpass)
        .add_system(rendering_system())
        .add_thread_local_fn(end_pbr_renderpass)
        .add_thread_local_fn(apply_haptic_feedback)
        .add_thread_local_fn(end_frame)
        .add_thread_local_fn(sync_debug_server)
        .build();

    let mut app = App::new(world, resources, schedule)?;
    app.run()?;
    Ok(())
}

fn add_pointer(
    colour: Colour,
    models: &HashMap<String, World>,
    world: &mut World,
    vulkan_context: &hotham::resources::vulkan_context::VulkanContext,
    render_context: &RenderContext,
) {
    let (handedness, model_name) = match colour {
        Colour::Red => (Handedness::Left, "Red Pointer"),
        Colour::Blue => (Handedness::Right, "Blue Pointer"),
    };
    let pointer = add_model_to_world(
        model_name,
        models,
        world,
        None,
        vulkan_context,
        &render_context.descriptor_set_layouts,
    )
    .unwrap();
    {
        let mut pointer_entry = world.entry(pointer).unwrap();
        pointer_entry.add_component(Pointer {
            handedness,
            trigger_value: 0.,
        });
        pointer_entry.add_component(colour);
    }
}

fn add_saber(
    colour: Colour,
    models: &HashMap<String, World>,
    world: &mut World,
    vulkan_context: &hotham::resources::vulkan_context::VulkanContext,
    render_context: &RenderContext,
    physics_context: &mut PhysicsContext,
) {
    let model_name = match colour {
        Colour::Red => "Red Saber",
        Colour::Blue => "Blue Saber",
    };
    let saber = add_model_to_world(
        model_name,
        models,
        world,
        None,
        vulkan_context,
        &render_context.descriptor_set_layouts,
    )
    .unwrap();
    {
        add_saber_physics(world, physics_context, saber);
        let mut saber_entry = world.entry(saber).unwrap();
        saber_entry.add_component(Saber {});
        saber_entry.add_component(colour);
    }
}

fn add_environment_models(
    models: &HashMap<String, World>,
    world: &mut World,
    vulkan_context: &hotham::resources::vulkan_context::VulkanContext,
    render_context: &RenderContext,
) {
    let _environment = add_model_to_world(
        "Environment",
        models,
        world,
        None,
        vulkan_context,
        &render_context.descriptor_set_layouts,
    )
    .unwrap();
    let _ramp = add_model_to_world(
        "Ramp",
        models,
        world,
        None,
        vulkan_context,
        &render_context.descriptor_set_layouts,
    )
    .unwrap();
}
