use legion::{system, world::SubWorld, EntityStore};

use crate::components::{animation_controller::AnimationController, AnimationTarget, Transform};

#[system(for_each)]
#[read_component(AnimationController)]
pub fn animation(
    transform: &mut Transform,
    animation_target: &AnimationTarget,
    world: &mut SubWorld,
) {
    let controller_entity = world.entry_ref(animation_target.controller).unwrap();
    let controller = controller_entity
        .get_component::<AnimationController>()
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

#[cfg(test)]
mod tests {
    use legion::{component, IntoQuery, Schedule, World};

    use crate::{
        add_model_to_world,
        gltf_loader::load_models_from_gltf,
        resources::{render_context::create_descriptor_set_layouts, VulkanContext},
    };

    use super::*;
    #[test]
    pub fn animation_test() {
        let vulkan_context = VulkanContext::testing().unwrap();
        let set_layouts = create_descriptor_set_layouts(&vulkan_context).unwrap();

        let data: Vec<(&[u8], &[u8])> = vec![(
            include_bytes!("../../../hotham-asteroid/assets/left_hand.gltf"),
            include_bytes!("../../../hotham-asteroid/assets/left_hand.bin"),
        )];
        let models = load_models_from_gltf(data, &vulkan_context, set_layouts.mesh_layout).unwrap();

        let mut world = World::default();

        // Add the left hand
        let left_hand = add_model_to_world("Left Hand", &models, &mut world, None).unwrap();
        {
            let mut left_hand_entry = world.entry_mut(left_hand).unwrap();
            let left_hand_controller = left_hand_entry
                .get_component_mut::<AnimationController>()
                .unwrap();
            left_hand_controller.blend_from = 0;
            left_hand_controller.blend_from = 1;
            left_hand_controller.blend_amount = 0.5;
        }

        // Collect all the transforms in the world so we can compare them later.
        let mut query = <&Transform>::query().filter(component::<AnimationTarget>());
        let transforms_before = query
            .iter(&world)
            .map(Clone::clone)
            .collect::<Vec<Transform>>();

        let mut resources = Default::default();
        let mut schedule = Schedule::builder().add_system(animation_system()).build();
        schedule.execute(&mut world, &mut resources);

        let transforms_after = query
            .iter(&world)
            .map(Clone::clone)
            .collect::<Vec<Transform>>();

        assert_ne!(transforms_before, transforms_after);
    }
}
