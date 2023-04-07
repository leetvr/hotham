use hotham::{asset_importer::add_model_to_world, components::LocalTransform, hecs::World, Engine};

const AXES_SPACE: [AxesSpace; 5] = [
    AxesSpace::LeftGrip,
    AxesSpace::LeftAim,
    AxesSpace::RightGrip,
    AxesSpace::RightAim,
    AxesSpace::Hmd,
];

pub enum AxesSpace {
    LeftGrip,
    LeftAim,
    RightGrip,
    RightAim,
    Hmd,
}

pub struct Axes {
    space: AxesSpace,
}

pub fn add_axes(models: &std::collections::HashMap<String, World>, world: &mut World) {
    for space in AXES_SPACE {
        let entity = add_model_to_world("Axes", models, world, None).unwrap();
        world.insert_one(entity, Axes { space }).unwrap();
    }
}

pub fn inverse_kinematics_system(engine: &mut Engine) {
    let world = &mut engine.world;
    let input_context = &engine.input_context;
    for (_, (local_transform, axes)) in world
        .query_mut::<(&mut LocalTransform, &Axes)>()
        .into_iter()
    {
        local_transform.update_from_affine(&match axes.space {
            AxesSpace::LeftGrip => input_context.left.stage_from_grip(),
            AxesSpace::LeftAim => input_context.left.stage_from_aim(),
            AxesSpace::RightGrip => input_context.right.stage_from_grip(),
            AxesSpace::RightAim => input_context.right.stage_from_aim(),
            AxesSpace::Hmd => input_context.hmd.hmd_in_stage(),
        });
    }
}
