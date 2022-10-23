mod custom_render_context;
mod custom_rendering;
mod hologram;
mod navigation;

use custom_render_context::CustomRenderContext;
use custom_rendering::custom_rendering_system;
use hologram::{Hologram, HologramData};
use hotham::{
    asset_importer::{self, add_model_to_world},
    components::{
        hand::Handedness, physics::SharedShape, Collider, Grabbable, LocalTransform, Mesh,
    },
    glam::{Affine3A, Mat4, Quat, Vec3},
    hecs::World,
    systems::{
        animation_system, debug::debug_system, grabbing_system, hands::add_hand, hands_system,
        physics_system, skinning::skinning_system, update_global_transform_system,
        update_global_transform_with_parent_system,
    },
    xr, Engine, HothamResult, TickData,
};
use navigation::navigation_system;

#[derive(Clone, Debug, Default)]
/// The state is used for manipulating the stage transform
pub struct State {
    global_from_left_grip: Option<Affine3A>,
    global_from_right_grip: Option<Affine3A>,
    scale: Option<f32>,
}

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
pub fn main() {
    println!("[HOTHAM_COMPLEX_SCENE] MAIN!");
    real_main().expect("Error running app!");
    println!("[HOTHAM_COMPLEX_SCENE] FINISHED! Goodbye!");
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
    if tick_data.current_state == xr::SessionState::FOCUSED {
        hands_system(engine);
        grabbing_system(engine);
        physics_system(engine);
        animation_system(engine);
        navigation_system(engine, state);
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

fn init(engine: &mut Engine) -> Result<(), hotham::HothamError> {
    let render_context = &mut engine.render_context;
    let vulkan_context = &mut engine.vulkan_context;
    let world = &mut engine.world;

    let mut glb_buffers: Vec<&[u8]> = vec![
        include_bytes!("../../../test_assets/left_hand.glb"),
        include_bytes!("../../../test_assets/right_hand.glb"),
        include_bytes!("../../../test_assets/sphere.glb"),
    ];

    #[cfg(target_os = "android")]
    glb_buffers.push(include_bytes!(
        "../../../test_assets/damaged_helmet_squished.glb"
    ));

    #[cfg(not(target_os = "android"))]
    glb_buffers.push(include_bytes!("../../../test_assets/damaged_helmet.glb"));

    let models =
        asset_importer::load_models_from_glb(&glb_buffers, vulkan_context, render_context)?;
    add_helmet(&models, world, [-1., 1.4, -1.].into());
    add_helmet(&models, world, [1., 1.4, -1.].into());
    add_hand(&models, Handedness::Left, world);
    add_hand(&models, Handedness::Right, world);
    add_quadric(
        &models,
        world,
        &LocalTransform {
            translation: [1.0, 3.0, -1.5].into(),
            rotation: Quat::IDENTITY,
            scale: [0.5, 0.5, 0.5].into(),
        },
        0.5,
        HologramData {
            surface_q_in_local: Mat4::from_diagonal([1.0, 1.0, 1.0, -1.0].into()),
            bounds_q_in_local: Mat4::from_diagonal([0.0, 0.0, 0.0, 0.0].into()),
            uv_from_local: Mat4::IDENTITY,
        },
    );
    add_quadric(
        &models,
        world,
        &LocalTransform {
            translation: [-1.0, 3.0, -1.5].into(),
            rotation: Quat::IDENTITY,
            scale: [0.5, 0.5, 0.5].into(),
        },
        0.5,
        HologramData {
            surface_q_in_local: Mat4::from_diagonal([1.0, 1.0, 0.0, -1.0].into()),
            bounds_q_in_local: Mat4::from_diagonal([0.0, 0.0, 1.0, -1.0].into()),
            uv_from_local: Mat4::IDENTITY,
        },
    );

    Ok(())
}

fn add_helmet(
    models: &std::collections::HashMap<String, World>,
    world: &mut World,
    translation: Vec3,
) {
    let helmet = add_model_to_world("Damaged Helmet", models, world, None)
        .expect("Could not find Damaged Helmet");

    {
        let mut local_transform = world.get::<&mut LocalTransform>(helmet).unwrap();
        local_transform.translation = translation;
        local_transform.scale = [0.5, 0.5, 0.5].into();
    }

    let collider = Collider::new(SharedShape::ball(0.35));

    world.insert(helmet, (collider, Grabbable {})).unwrap();
}

fn add_quadric(
    models: &std::collections::HashMap<String, World>,
    world: &mut World,
    local_transform: &LocalTransform,
    ball_radius: f32,
    hologram_data: HologramData,
) {
    let entity = add_model_to_world("Sphere", models, world, None).expect("Could not find Sphere");
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
}
