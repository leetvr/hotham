use hotham::{
    asset_importer::add_model_to_world,
    components::{hand::Handedness, LocalTransform, Stage},
    glam::{vec3, Affine3A, Quat, Vec3},
    hecs::World,
    Engine,
};
use inline_tweak::tweak;

#[derive(Clone)]
pub struct IkSelectorBackground {
    /// Which hand is this referring to?
    pub handedness: Handedness,
}

pub struct IkSelectorKnob {
    /// Which hand is this referring to?
    pub handedness: Handedness,
}

pub fn add_ik_selectors(models: &std::collections::HashMap<String, World>, world: &mut World) {
    let stages = world
        .query::<&Stage>()
        .iter()
        .map(|(entity, _)| entity)
        .collect::<Vec<_>>();
    for parent in stages {
        for handedness in [Handedness::Left, Handedness::Right] {
            let selector_background =
                add_model_to_world("KnobBackground", models, world, Some(parent)).unwrap();
            world
                .insert_one(selector_background, IkSelectorBackground { handedness })
                .unwrap();
            let selector_knob =
                add_model_to_world("Knob", models, world, Some(selector_background)).unwrap();
            world
                .insert_one(selector_knob, IkSelectorKnob { handedness })
                .unwrap();
        }
    }
}

pub fn ik_selector_system(engine: &mut Engine) {
    let world = &mut engine.world;
    let input_context = &engine.input_context;
    let knob_translation_scale = tweak!(0.04);
    let angle_deg: f32 = tweak!(-15.0);
    let rotation = Quat::from_axis_angle(Vec3::X, angle_deg.to_radians());
    let translation = vec3(0.0, tweak!(0.0), tweak!(-0.05));
    let selector_in_grip = Affine3A::from_rotation_translation(rotation, translation);
    for (_, (ik_selector_background, local_transform)) in world
        .query::<(&mut IkSelectorBackground, &mut LocalTransform)>()
        .iter()
    {
        let stage_from_grip = match ik_selector_background.handedness {
            Handedness::Left => input_context.left.stage_from_grip(),
            Handedness::Right => input_context.right.stage_from_grip(),
        };
        let selector_in_stage = stage_from_grip * selector_in_grip;
        local_transform.update_from_affine(&selector_in_stage);
    }
    for (_, (ik_selector_knob, local_transform)) in world
        .query::<(&mut IkSelectorKnob, &mut LocalTransform)>()
        .iter()
    {
        let thumbstick_xy = match ik_selector_knob.handedness {
            Handedness::Left => input_context.left.thumbstick_xy(),
            Handedness::Right => input_context.right.thumbstick_xy(),
        };
        let knob_in_background = Affine3A::from_translation(
            knob_translation_scale * vec3(thumbstick_xy.x, 0.0, -thumbstick_xy.y),
        );
        local_transform.update_from_affine(&knob_in_background);
    }
}
