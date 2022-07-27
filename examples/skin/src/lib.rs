use hotham::{
    asset_importer::{self, add_model_to_world, Models},
    components::{Transform, Info, Parent},
    hecs::{World, With, CommandBuffer},
    systems::{
        rendering::rendering_system, skinning::skinning_system,
        update_parent_transform_matrix_system,
        update_transform_matrix_system, Queries,
    },
    xr, Engine, HothamResult, TickData, nalgebra::{UnitQuaternion, Translation3},
};

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
pub fn main() {
    real_main().expect("Error running app!");
}

pub fn real_main() -> HothamResult<()> {
    let mut engine = Engine::new();
    let mut world = init(&mut engine)?;
    let mut queries = Default::default();

    while let Ok(tick_data) = engine.update() {
        tick(tick_data, &mut engine, &mut world, &mut queries);
        engine.finish()?;
    }

    Ok(())
}

fn init(engine: &mut Engine) -> Result<World, hotham::HothamError> {
    let render_context = &mut engine.render_context;

    let vulkan_context = &mut engine.vulkan_context;
    let _physics_context = &mut engine.physics_context;
    let mut world = World::default();

    let glb_buffers: Vec<&[u8]> = vec![
        include_bytes!("../../../test_assets/box_armature.glb"),
    ];

    let models =
        asset_importer::load_models_from_glb(&glb_buffers, vulkan_context, render_context)?;

    add_box(&models, &mut world);

    Ok(world)
}

fn tick(
    tick_data: TickData,
    engine: &mut Engine,
    world: &mut World,
    queries: &mut Queries,
) {
    let xr_context = &mut engine.xr_context;
    let vulkan_context = &engine.vulkan_context;
    let render_context = &mut engine.render_context;
    let _physics_context = &mut engine.physics_context;

    if tick_data.current_state == xr::SessionState::FOCUSED {
        update_transform_matrix_system(&mut queries.update_transform_matrix_query, world);
        update_parent_transform_matrix_system(
            &mut queries.parent_query,
            &mut queries.roots_query,
            world,
        );
        box_system(world);
        skinning_system(&mut queries.skins_query, world, render_context);
    }

    let views = xr_context.update_views();
    rendering_system(
        &mut queries.rendering_query,
        world,
        vulkan_context,
        render_context,
        views,
        tick_data.swapchain_image_index,
    );
}

struct Armature;

fn add_box(
    models: &Models,
    world: &mut World,
) {
    let _box_entity = add_model_to_world("Armature", models, world, None).unwrap();
    // world.insert_one(box_entity, Armature{}).unwrap();

    let mut cmd_buffer = CommandBuffer::new();
    for (e, (_info, _parent)) in world.query::<(&Info, &mut Parent)>().iter() {
        cmd_buffer.remove::<(Parent,)>(e);
        cmd_buffer.insert(e, (Armature,));
    }
    cmd_buffer.run_on(world);

    // rotate one of the bones a bit
    for (_e, (info, transform)) in world.query::<(&Info, &mut Transform)>().iter() {
        if info.name == "Bone.002" {
            transform.rotation = UnitQuaternion::from_euler_angles(0.1, 0.2, 0.3);
        }
    }

    // move it backwards so we can see it
    for (_e, transform) in world.query_mut::<With<Armature, &mut Transform>>() {
        let offset: Translation3<f32> = [0.0, 0.0, -5.0].into();
        transform.translation += offset.vector;
    }
}

fn box_system(world: &mut World) {
    // rotate one of the bones a bit
    for (_e, (info, transform)) in world.query::<(&Info, &mut Transform)>().iter() {
        if info.name == "Bone.002" {
            transform.rotation *= UnitQuaternion::from_euler_angles(0.001, 0.002, 0.003);
        }
    }
}
