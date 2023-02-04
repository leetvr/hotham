mod custom_render_context;
mod custom_rendering;
mod hologram;
mod surface_solver;

use custom_render_context::{create_quadrics_pipeline, CustomRenderContext};
use custom_rendering::custom_rendering_system;
use hologram::{Hologram, HologramData};
use hotham::{
    asset_importer::{self, add_model_to_world},
    components::{
        hand::Handedness, physics::SharedShape, Collider, Grabbable, LocalTransform, Mesh,
    },
    glam::{Mat4, Quat},
    hecs::{Entity, World},
    systems::{
        animation_system, debug::debug_system, grabbing_system, hands::add_hand, hands_system,
        physics_system, skinning::skinning_system, update_global_transform_system,
        update_global_transform_with_parent_system,
    },
    util::u8_to_u32,
    xr, Engine, HothamResult, TickData,
};
use hotham_examples::navigation::{navigation_system, State};
use surface_solver::{surface_solver_system, ControlPoints, HologramBackside};

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
pub fn main() {
    println!("[HOTHAM_CUSTOM_RENDERING_EXAMPLE] MAIN!");
    real_main().expect("Error running app!");
    println!("[HOTHAM_CUSTOM_RENDERING_EXAMPLE] FINISHED! Goodbye!");
}

pub fn real_main() -> HothamResult<()> {
    let mut engine = Engine::new();
    let mut custom_render_context = CustomRenderContext::new(&mut engine);
    let mut state = Default::default();
    init(&mut engine)?;

    while let Ok(tick_data) = engine.update() {
        tick(
            tick_data,
            &mut engine,
            &mut custom_render_context,
            &mut state,
        );
        engine.finish()?;
    }

    Ok(())
}

fn tick(
    tick_data: TickData,
    engine: &mut Engine,
    custom_render_context: &mut CustomRenderContext,
    state: &mut State,
) {
    if option_env!("HOTHAM_ASSET_SERVER_ADDRESS").is_some() {
        hot_reloading_system(engine, custom_render_context);
    }
    if tick_data.current_state == xr::SessionState::FOCUSED {
        hands_system(engine);
        grabbing_system(engine);
        physics_system(engine);
        animation_system(engine);
        navigation_system(engine, state);
        surface_solver_system(engine);
        update_global_transform_system(engine);
        update_global_transform_with_parent_system(engine);
        skinning_system(engine);
        debug_system(engine);
    }

    custom_rendering_system(
        engine,
        custom_render_context,
        tick_data.swapchain_image_index,
    );
}

fn hot_reloading_system(engine: &mut Engine, custom_render_context: &mut CustomRenderContext) {
    if engine
        .get_updated_assets()
        .iter()
        .any(|asset_updated| -> bool {
            match asset_updated.asset_id.as_str() {
                "examples/custom-rendering/src/shaders/quadric.vert.spv" => {
                    custom_render_context.vertex_shader_code =
                        u8_to_u32(asset_updated.asset_data.clone());
                    true
                }
                "examples/custom-rendering/src/shaders/quadric.frag.spv" => {
                    custom_render_context.fragment_shader_code =
                        u8_to_u32(asset_updated.asset_data.clone());
                    true
                }
                _ => false,
            }
        })
    {
        println!("[HOTHAM_CUSTOM_RENDERING_EXAMPLE] Recreating quadrics pipeline!");
        let vulkan_context = &mut engine.vulkan_context;
        let render_context = &engine.render_context;
        let quadrics_pipeline = create_quadrics_pipeline(
            vulkan_context,
            custom_render_context.quadrics_pipeline_layout,
            &render_context.render_area(),
            render_context.render_pass,
            custom_render_context.vertex_shader_code.as_slice(),
            custom_render_context.fragment_shader_code.as_slice(),
        );
        if let Ok(quadrics_pipeline) = quadrics_pipeline {
            custom_render_context.quadrics_pipeline = quadrics_pipeline;
        }
    }
}

fn init(engine: &mut Engine) -> Result<(), hotham::HothamError> {
    if option_env!("HOTHAM_ASSET_SERVER_ADDRESS").is_some() {
        let asset_list = vec![
            "examples/custom-rendering/src/shaders/quadric.frag.spv".into(),
            "examples/custom-rendering/src/shaders/quadric.vert.spv".into(),
        ];
        engine.watch_assets(asset_list);
    }
    let render_context = &mut engine.render_context;
    let vulkan_context = &mut engine.vulkan_context;
    let world = &mut engine.world;

    let glb_buffers: Vec<&[u8]> = vec![
        include_bytes!("../../../test_assets/left_hand.glb"),
        include_bytes!("../../../test_assets/right_hand.glb"),
        include_bytes!("../../../test_assets/hologram_templates.glb"),
    ];

    let models =
        asset_importer::load_models_from_glb(&glb_buffers, vulkan_context, render_context)?;
    add_hand(&models, Handedness::Left, world);
    add_hand(&models, Handedness::Right, world);
    let uv1_from_local = Mat4::from_diagonal([1.0, 1.0, 1.0, 0.1].into());
    let uv2_from_local = Mat4::from_cols_array(&[
        1.0, 0.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 0.0, 0.1, //
    ]);
    let uv3_from_local = Mat4::from_diagonal([0.0, 1.0, 0.0, 0.1].into());
    add_quadric(
        &models,
        "Sphere",
        world,
        &make_transform(-1.0, 1.4, -1.5, 0.5),
        0.5,
        HologramData {
            surface_q_in_local: Mat4::from_diagonal([1.0, 1.0, 1.0, -1.0].into()),
            bounds_q_in_local: Mat4::from_diagonal([0.0, 0.0, 0.0, 0.0].into()),
            uv_from_local: uv1_from_local,
        },
    );
    add_quadric(
        &models,
        "Cylinder",
        world,
        &make_transform(0.0, 1.4, -1.5, 0.5),
        0.5_f32.sqrt(),
        HologramData {
            surface_q_in_local: Mat4::from_diagonal([1.0, 0.0, 1.0, -1.0].into()),
            bounds_q_in_local: Mat4::from_diagonal([0.0, 1.0, 0.0, -1.0].into()),
            uv_from_local: uv1_from_local,
        },
    );
    add_quadric(
        &models,
        "Cylinder",
        world,
        &make_transform(1.0, 1.4, -1.5, 0.5),
        0.5_f32.sqrt(),
        HologramData {
            surface_q_in_local: Mat4::from_diagonal([1.0, -1.0 + 0.1, 1.0, -0.1].into()),
            bounds_q_in_local: Mat4::from_diagonal([0.0, 1.0, 0.0, -1.0].into()),
            uv_from_local: uv2_from_local,
        },
    );
    add_quadric(
        &models,
        "Cylinder",
        world,
        &make_transform(2.0, 1.4, -1.5, 0.5),
        0.5_f32.sqrt(),
        HologramData {
            surface_q_in_local: Mat4::from_diagonal([1.0, -1.0, 1.0, 0.0].into()),
            bounds_q_in_local: Mat4::from_diagonal([0.0, 1.0, 0.0, -1.0].into()),
            uv_from_local: uv2_from_local,
        },
    );
    add_quadric(
        &models,
        "Cylinder",
        world,
        &make_transform(3.0, 1.4, -1.5, 0.5),
        0.5_f32.sqrt(),
        HologramData {
            surface_q_in_local: Mat4::from_diagonal([1.0, -1.0 - 0.1, 1.0, 0.1].into()),
            bounds_q_in_local: Mat4::from_diagonal([0.0, 1.0, 0.0, -1.0].into()),
            uv_from_local: uv2_from_local,
        },
    );

    let target = add_quadric(
        &models,
        "Sphere",
        world,
        &make_transform(-1.0, 1.4, 1.5, 0.5),
        0.5,
        HologramData {
            surface_q_in_local: Mat4::from_diagonal([1.0, 1.0, 1.0, -1.0].into()),
            bounds_q_in_local: Mat4::from_diagonal([1.0, 1.0, 1.0, -1.0].into()),
            uv_from_local: uv2_from_local,
        },
    );

    let t_from_local = Mat4::from_translation([0.0, -1.0, 0.0].into());
    let t2_from_local = Mat4::from_translation([0.0, -0.5, 0.0].into());
    let entities = (0..6)
        .map(|i| {
            add_quadric(
                &models,
                "Cylinder",
                world,
                &make_transform(
                    -1.0 + 0.1 * (i & 1) as f32,
                    1.4,
                    1.5 + 0.1 * (i >> 1) as f32,
                    0.05,
                ),
                0.05,
                HologramData {
                    surface_q_in_local: t_from_local.transpose()
                        * Mat4::from_diagonal([1.0, -1.0, 1.0, 0.0].into())
                        * t_from_local,
                    bounds_q_in_local: t2_from_local.transpose()
                        * Mat4::from_diagonal([0.0, 1.0, 0.0, -0.5].into())
                        * t2_from_local,
                    uv_from_local: uv3_from_local,
                },
            )
        })
        .collect();

    let control_points = ControlPoints { entities };
    world.insert_one(target, control_points).unwrap();
    world.remove_one::<Grabbable>(target).unwrap();

    Ok(())
}

fn make_transform(x: f32, y: f32, z: f32, scale: f32) -> LocalTransform {
    LocalTransform {
        translation: [x, y, z].into(),
        rotation: Quat::IDENTITY,
        scale: [scale, scale, scale].into(),
    }
}

fn add_quadric(
    models: &std::collections::HashMap<String, World>,
    model_name: &str,
    world: &mut World,
    local_transform: &LocalTransform,
    ball_radius: f32,
    hologram_data: HologramData,
) -> Entity {
    let entity = add_model_to_world(model_name, models, world, None)
        .unwrap_or_else(|| panic!("Could not find {}", model_name));
    *world.get::<&mut LocalTransform>(entity).unwrap() = *local_transform;
    let collider = Collider::new(SharedShape::ball(ball_radius));
    let hologram_component = Hologram {
        mesh_data_handle: world.get::<&Mesh>(entity).unwrap().handle,
        hologram_data,
    };
    world
        .insert(entity, (collider, Grabbable {}, hologram_component))
        .unwrap();
    world.remove_one::<Mesh>(entity).unwrap();

    // Add second entity for the back surface
    let second_entity = add_model_to_world(model_name, models, world, Some(entity))
        .unwrap_or_else(|| panic!("Could not find {}", model_name));
    // Negate Q to flip the surface normal
    let mut hologram_component = hologram_component;
    hologram_component.hologram_data.surface_q_in_local *= -1.0;
    world
        .insert(
            second_entity,
            (hologram_component, HologramBackside { entity }),
        )
        .unwrap();
    world.remove_one::<Mesh>(second_entity).unwrap();

    entity
}
