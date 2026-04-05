use crate::{
    components::{animation_controller::AnimationController, LocalTransform},
    Engine,
};

/// Animation system
/// Walks through each AnimationController and applies the appropriate animation to its targets.
pub fn animation_system(engine: &mut Engine) {
    animation_system_inner(&mut engine.world);
}

fn animation_system_inner(world: &mut hecs::World) {
    for (_, controller) in world.query::<&AnimationController>().iter() {
        let Some(animation_state) = controller.current_animation() else {
            continue;
        };

        for target in &animation_state.targets {
            let mut local_transform = world.get::<&mut LocalTransform>(target.target).unwrap();
        }
    }
}

#[cfg(target_os = "windows")]
#[cfg(test)]
mod tests {
    use crate::{
        asset_importer::{add_model_to_world, load_models_from_glb},
        contexts::RenderContext,
    };

    use super::*;
    #[test]
    pub fn animation_test() {
        let (mut render_context, vulkan_context) = RenderContext::testing();

        let data: Vec<&[u8]> = vec![include_bytes!("../../../test_assets/left_hand.glb")];
        let models = load_models_from_glb(&data, &vulkan_context, &mut render_context).unwrap();
        let mut world = hecs::World::new();

        // Add the left hand
        let left_hand = add_model_to_world("Left Hand", &models, &mut world, None).unwrap();
        {
            let mut left_hand_controller =
                world.get::<&mut AnimationController>(left_hand).unwrap();
            left_hand_controller.blend_from = 0;
            left_hand_controller.blend_to = 1;
            left_hand_controller.blend_amount = 0.0;
        }

        // Collect all the transforms in the world so we can compare them later.
        let transforms_before = world
            .query_mut::<&LocalTransform>()
            .into_iter()
            .map(|r| *r.1)
            .collect::<Vec<LocalTransform>>();

        // Run the animation system
        animation_system_inner(&mut world);

        // Collect all the transforms after the system has been run.
        let transforms_after = world
            .query_mut::<&LocalTransform>()
            .into_iter()
            .map(|r| *r.1)
            .collect::<Vec<LocalTransform>>();

        // Make sure our transforms have been modified!
        assert_ne!(transforms_before, transforms_after);
    }
}
