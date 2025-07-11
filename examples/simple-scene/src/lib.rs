use hotham::{
    asset_importer::{self, add_model_to_world},
    components::{
        hand::Handedness,
        physics::{BodyType, SharedShape},
        Collider, GlobalTransform, Info, LocalTransform, Mesh, RigidBody,
    },
    hecs::World,
    na,
    systems::{
        animation_system, debug::debug_system, grabbing_system, hands::add_hand, hands_system,
        physics_system, rendering::rendering_system, skinning::skinning_system,
        update_global_transform_system,
    },
    xr, Engine, HothamResult, TickData,
};
use hotham_editor_protocol::scene::{EditorEntity, EditorUpdates, Transform};
use log::{debug, info};

#[cfg(windows)]
use uds_windows::UnixStream;

#[cfg(unix)]
use std::os::unix::net::UnixStream;

#[derive(Clone, Debug, Default)]
/// Most Hotham applications will want to keep track of some sort of state.
/// However, this _simple_ scene doesn't have any, so this is just left here to let you know that
/// this is something you'd probably want to do!
struct State {}

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
pub fn main() {
    println!("[HOTHAM_SIMPLE_SCENE] MAIN!");
    real_main().expect("Error running app!");
    println!("[HOTHAM_SIMPLE_SCENE] FINISHED! Goodbye!");
}

pub fn real_main() -> HothamResult<()> {
    env_logger::builder()
        .filter_module("hotham-openxr-client", log::LevelFilter::Trace)
        .filter_module("simple_scene_example", log::LevelFilter::Trace)
        .init();

    info!("Initialising Simple Scene example..");

    #[cfg(feature = "editor")]
    let mut editor = {
        use hotham_editor_protocol::EditorClient;

        info!("Connecting to editor..");
        let stream = UnixStream::connect("hotham_editor.socket")?;
        EditorClient::new(stream)
    };

    info!("Building engine..");
    let mut engine = Engine::new();
    info!("..done!");

    info!("Initialising app..");
    let mut state = Default::default();
    init(&mut engine)?;
    info!("Done! Entering main loop..");

    while let Ok(tick_data) = engine.update() {
        #[cfg(feature = "editor")]
        sync_with_editor(&mut engine.world, &mut editor)?;

        tick(tick_data, &mut engine, &mut state);
        engine.finish()?;
    }

    Ok(())
}

fn sync_with_editor(
    world: &mut World,
    editor: &mut hotham_editor_protocol::EditorClient<UnixStream>,
) -> HothamResult<()> {
    use hotham::hecs::Entity;
    let entities = world
        .query_mut::<(&GlobalTransform, &Info)>()
        .with::<&Mesh>()
        .into_iter()
        .map(|(entity, (transform, info))| {
            let (_, _, translation) = transform.to_scale_rotation_translation();
            EditorEntity {
                name: info.name.clone(),
                id: entity.to_bits().get(),
                transform: Transform {
                    translation: translation.into(),
                },
            }
        })
        .collect();

    let scene = hotham_editor_protocol::scene::Scene {
        name: "Simple Scene".to_string(),
        entities,
    };

    editor.send_json(&scene).unwrap(); // TODO: error types

    let editor_updates: EditorUpdates = editor.get_json().unwrap(); // TODO: error types
    for entity in editor_updates.entity_updates {
        debug!("Received update: {entity:?}");
        let mut entity_transform = world
            .entity(Entity::from_bits(entity.id).unwrap())
            .unwrap()
            .get::<&mut LocalTransform>()
            .unwrap();
        entity_transform.translation = entity.transform.translation.into();
    }

    Ok(())
}

fn tick(tick_data: TickData, engine: &mut Engine, _state: &mut State) {
    if tick_data.current_state == xr::SessionState::FOCUSED {
        hands_system(engine);
        grabbing_system(engine);
        physics_system(engine);
        animation_system(engine);
        update_global_transform_system(engine);
        skinning_system(engine);
        debug_system(engine);
    }

    rendering_system(engine, tick_data.swapchain_image_index);
}

fn init(engine: &mut Engine) -> Result<(), hotham::HothamError> {
    let render_context = &mut engine.render_context;
    let vulkan_context = &mut engine.vulkan_context;
    let world = &mut engine.world;

    let mut glb_buffers: Vec<&[u8]> = vec![
        include_bytes!("../../../test_assets/floor.glb"),
        include_bytes!("../../../test_assets/left_hand.glb"),
        include_bytes!("../../../test_assets/right_hand.glb"),
    ];
    let models =
        asset_importer::load_models_from_glb(&glb_buffers, vulkan_context, render_context)?;
    add_floor(&models, world);
    add_hand(&models, Handedness::Left, world);
    add_hand(&models, Handedness::Right, world);

    #[cfg(target_os = "android")]
    glb_buffers.push(include_bytes!(
        "../../../test_assets/damaged_helmet_squished.glb"
    ));

    #[cfg(not(target_os = "android"))]
    glb_buffers.push(include_bytes!("../../../test_assets/damaged_helmet.glb"));

    let models =
        asset_importer::load_models_from_glb(&glb_buffers, vulkan_context, render_context)?;
    add_helmet(&models, world);
    add_model_to_world("Cube", &models, world, None);

    // Update global transforms from local transforms before physics_system gets confused
    update_global_transform_system(engine);

    Ok(())
}

fn add_floor(models: &std::collections::HashMap<String, World>, world: &mut World) {
    let entity = add_model_to_world("Floor", models, world, None).expect("Could not find Floor");
    let collider = Collider::new(SharedShape::halfspace(na::Vector3::y_axis()));
    let rigid_body = RigidBody {
        body_type: BodyType::Fixed,
        ..Default::default()
    };
    world.insert(entity, (collider, rigid_body)).unwrap();
}

fn add_helmet(models: &std::collections::HashMap<String, World>, world: &mut World) {
    let helmet = add_model_to_world("Damaged Helmet", models, world, None)
        .expect("Could not find Damaged Helmet");

    {
        let mut local_transform = world.get::<&mut LocalTransform>(helmet).unwrap();
        local_transform.translation.z = -1.;
        local_transform.translation.y = 1.4;
        local_transform.scale = [0.5, 0.5, 0.5].into();
    }

    let collider = Collider::new(SharedShape::ball(0.35));

    world.insert_one(helmet, collider).unwrap();
}
