use std::ops::Mul;

use hotham::{
    components::LocalTransform,
    contexts::InputContext,
    glam::{Affine3A, Quat, Vec3, Vec3A},
    hecs::{self, World},
    Engine,
};

#[derive(Clone, Debug, Default)]
/// This state is used for manipulating the stage transform.
pub struct State {
    global_from_left_grip: Option<Affine3A>,
    global_from_right_grip: Option<Affine3A>,
    scale: Option<f32>,
}

/// Navigation system
/// Allows the player to navigate by grabbing empty space with their hands.
pub fn navigation_system(engine: &mut Engine, state: &mut State) {
    let world = &mut engine.world;
    let input_context = &mut engine.input_context;
    navigation_system_inner(world, input_context, engine.stage_entity, state);
}

fn navigation_system_inner(
    world: &mut World,
    input_context: &mut InputContext,
    stage_entity: hecs::Entity,
    state: &mut State,
) {
    // Get the stage transform.
    let mut stage_transform = world.get::<&mut LocalTransform>(stage_entity).unwrap();
    let global_from_stage = stage_transform.to_affine();

    // Get the hand transforms.
    let stage_from_left_grip = input_context.left.stage_from_grip();
    let stage_from_right_grip = input_context.right.stage_from_grip();

    // Update grip states.
    if input_context.left.y_button_just_pressed() {
        state.global_from_left_grip = Some(global_from_stage * stage_from_left_grip);
    }
    if input_context.right.b_button_just_pressed() {
        state.global_from_right_grip = Some(global_from_stage * stage_from_right_grip);
    }
    if input_context.right.b_button() && input_context.left.y_button_just_released() {
        // Handle when going from two grips to one
        state.global_from_right_grip = Some(global_from_stage * stage_from_right_grip);
    }
    if !input_context.left.y_button() {
        state.global_from_left_grip = None;
        state.scale = None;
    }
    if !input_context.right.b_button() {
        state.global_from_right_grip = None;
    }

    // Adjust global_from_stage so that global_from_grip stays fixed.
    // global_from_stage * stage_from_grip = global_from_stored_grip
    // global_from_stage = global_from_stored_grip * grip_from_stage
    match (
        state.global_from_left_grip,
        state.global_from_right_grip,
        state.scale,
    ) {
        (Some(global_from_stored_left_grip), None, None) => {
            stage_transform.update_from_affine(
                &(global_from_stored_left_grip * stage_from_left_grip.inverse()),
            );
            constrain_up_axis(
                &mut stage_transform,
                &global_from_stored_left_grip,
                &stage_from_left_grip,
            );
        }
        (Some(global_from_stored_left_grip), None, Some(scale)) => {
            stage_transform.update_from_affine(
                &(global_from_stored_left_grip
                    * Affine3A::from_scale(Vec3::new(scale, scale, scale))
                    * stage_from_left_grip.inverse()),
            );
            constrain_up_axis(
                &mut stage_transform,
                &global_from_stored_left_grip,
                &stage_from_left_grip,
            );
        }
        (None, Some(global_from_stored_right_grip), _) => {
            stage_transform.update_from_affine(
                &(global_from_stored_right_grip * stage_from_right_grip.inverse()),
            );
            constrain_up_axis(
                &mut stage_transform,
                &global_from_stored_right_grip,
                &stage_from_right_grip,
            );
        }
        (Some(global_from_stored_left_grip), Some(global_from_stored_right_grip), _) => {
            // Gripping with both hands allows scaling the scene
            // The first hand acts as an anchor and the second hand only scales the scene.
            let stored_left_grip_in_global = global_from_stored_left_grip.translation;
            let stored_right_grip_in_global = global_from_stored_right_grip.translation;
            let left_grip_in_stage = stage_from_left_grip.translation;
            let right_grip_in_stage = stage_from_right_grip.translation;

            let unscaled_global_from_stage =
                global_from_stored_left_grip * stage_from_left_grip.inverse();
            let left_grip_in_unscaled_global =
                unscaled_global_from_stage.transform_point3a(left_grip_in_stage);
            let right_grip_in_unscaled_global =
                unscaled_global_from_stage.transform_point3a(right_grip_in_stage);
            let stored_dist_in_global =
                stored_left_grip_in_global.distance(stored_right_grip_in_global);
            let dist_in_unscaled_global =
                left_grip_in_unscaled_global.distance(right_grip_in_unscaled_global);
            let scale = stored_dist_in_global / dist_in_unscaled_global;

            // Remember scale for when one grip gets released.
            state.scale = Some(scale);

            // Let left hand be dominant for now.
            let stored_left_grip_from_left_grip =
                Affine3A::from_scale(Vec3::new(scale, scale, scale));

            stage_transform.update_from_affine(
                &(global_from_stored_left_grip
                    * stored_left_grip_from_left_grip
                    * stage_from_left_grip.inverse()),
            );
            constrain_up_axis(
                &mut stage_transform,
                &global_from_stored_left_grip,
                &stage_from_left_grip,
            );
        }
        (None, None, _) => (),
    };
}

fn constrain_up_axis(
    stage_transform: &mut LocalTransform,
    global_from_stored_grip: &Affine3A,
    stage_from_grip: &Affine3A,
) {
    let transformed_y = stage_transform.rotation.mul_vec3a(Vec3A::Y);
    let angle = transformed_y.dot(Vec3A::Y).acos();
    if angle.abs() > 0.0001 {
        let axis = transformed_y.cross(Vec3A::Y).normalize();
        let rotation_compensation = Quat::from_axis_angle(axis.into(), angle);
        let constrained_rotation = rotation_compensation * stage_transform.rotation;
        let constrained_translation = global_from_stored_grip.translation
            + constrained_rotation
                * (-stage_from_grip
                    .translation
                    .mul(Vec3A::from(stage_transform.scale)));

        stage_transform.rotation = constrained_rotation;
        stage_transform.translation = constrained_translation.into();
    }
}
