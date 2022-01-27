use crate::components::{animation_controller::AnimationController, AnimationTarget, Transform};
use hecs::{PreparedQuery, World};

pub fn animation_system(
    query: &mut PreparedQuery<(&mut AnimationTarget, &mut Transform)>,
    world: &mut World,
) {
    for (_, (animation_target, transform)) in query.query_mut(world) {
        let controller = world
            .get::<&AnimationController>(animation_target.controller)
            .unwrap();
        let blend_from = controller.blend_from;
        let blend_to = controller.blend_to;
        let blend_amount = controller.blend_amount;

        let transform_from = animation_target.animations[blend_from][0];
        let transform_to = animation_target.animations[blend_to][0];

        transform.translation = transform_from
            .translation
            .lerp(&transform_to.translation, blend_amount);
        transform.rotation = transform_from
            .rotation
            .slerp(&transform_to.rotation, blend_amount);
        transform.scale = transform_from.scale.lerp(&transform_to.scale, blend_amount);
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        gltf_loader::{add_model_to_world, load_models_from_glb},
        resources::{render_context::create_descriptor_set_layouts, VulkanContext},
    };

    use super::*;
    #[test]
    pub fn animation_test() {
        let vulkan_context = VulkanContext::testing().unwrap();
        let set_layouts = create_descriptor_set_layouts(&vulkan_context).unwrap();

        let data: Vec<&[u8]> = vec![include_bytes!("../../../test_assets/left_hand.glb")];
        let models = load_models_from_glb(&data, &vulkan_context, &set_layouts).unwrap();
        let query = PreparedQuery::<(&mut AnimationTarget, &mut Transform)>::default();

        let mut world = World::new();

        // Add the left hand
        let left_hand = add_model_to_world(
            "Left Hand",
            &models,
            &mut world,
            None,
            &vulkan_context,
            &set_layouts,
        )
        .unwrap();
        {
            let mut left_hand_controller = world
                .get_mut::<&mut AnimationController>(left_hand)
                .unwrap();
            left_hand_controller.blend_from = 0;
            left_hand_controller.blend_from = 1;
            left_hand_controller.blend_amount = 0.5;
        }

        // Collect all the transforms in the world so we can compare them later.
        let transforms_before = query
            .query_mut(&mut world)
            .into_iter()
            .map(|(_, (_, t))| t.clone())
            .collect::<Vec<Transform>>();

        // Run the animation system
        animation_system(&mut query, &mut world);

        // Collect all the transforms after the system has been run.
        let transforms_after = query
            .query_mut(&mut world)
            .into_iter()
            .map(|(_, (_, t))| t.clone())
            .collect::<Vec<Transform>>();

        // Make sure our transforms have been modified!
        assert_ne!(transforms_before, transforms_after);
    }
}
