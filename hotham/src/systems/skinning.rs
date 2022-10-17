use hecs::World;
use render_context::RenderContext;

use crate::{
    components::{GlobalTransform, Skin},
    contexts::render_context,
    Engine,
};

/// Skinning system
/// Walks through each joint in the system and builds up the `joint_matrices` that will be sent to the vertex shader
pub fn skinning_system(engine: &mut Engine) {
    let world = &mut engine.world;
    let render_context = &mut engine.render_context;
    skinning_system_inner(world, render_context);
}

fn skinning_system_inner(world: &mut World, render_context: &mut RenderContext) {
    for (_, (skin, global_transform)) in world.query::<(&Skin, &GlobalTransform)>().iter() {
        let buffer = unsafe { render_context.resources.skins_buffer.as_slice_mut() };
        let joint_matrices = &mut buffer[skin.id as usize];
        let local_from_global = global_transform.0.inverse();

        for (n, (joint, joint_from_mesh)) in skin
            .joints
            .iter()
            .zip(skin.inverse_bind_matrices.iter())
            .enumerate()
        {
            let global_from_joint = world.get::<&GlobalTransform>(*joint).unwrap().0;
            let local_from_mesh = local_from_global * global_from_joint * *joint_from_mesh;
            joint_matrices[n] = local_from_mesh.into();
        }
    }
}

#[cfg(target_os = "windows")]
#[cfg(test)]
mod tests {

    use std::io::Write;

    use crate::{
        components::{Info, Skin},
        util::get_world_with_hands,
    };

    use super::*;
    use approx::relative_eq;
    use glam::{Affine3A, Mat4};

    #[test]
    pub fn test_hand_skinning() {
        let (mut render_context, vulkan_context) = RenderContext::testing();
        let mut world = get_world_with_hands(&vulkan_context, &mut render_context);

        skinning_system_inner(&mut world, &mut render_context);

        assert!(verify_matrices(&world, &render_context));

        // Muck all the joints up
        for (_, skin) in world.query::<&Skin>().iter() {
            for joint in &skin.joints {
                let mut global_transform = world.get::<&mut GlobalTransform>(*joint).unwrap();
                global_transform.0 = Affine3A::ZERO;
            }
        }
        skinning_system_inner(&mut world, &mut render_context);

        // TODO: This test is broken: https://github.com/leetvr/hotham/issues/370
        // assert!(verify_matrices(&world, &render_context));
    }

    fn verify_matrices(world: &World, render_context: &RenderContext) -> bool {
        let mut called = 0;
        for (_, (skin, info)) in world.query::<(&Skin, &Info)>().iter() {
            let correct_matrices: Vec<Mat4> = if info.name == "hands:Lhand" {
                serde_json::from_slice(include_bytes!(
                    "../../../test_assets/left_hand_skinned_matrices.json"
                ))
                .unwrap()
            } else {
                serde_json::from_slice(include_bytes!(
                    "../../../test_assets/right_hand_skinned_matrices.json"
                ))
                .unwrap()
            };
            let buffer = unsafe { render_context.resources.skins_buffer.as_slice() };
            let joint_matrices = &buffer[skin.id as usize];

            for i in 0..correct_matrices.len() {
                let expected = correct_matrices[i];
                let actual = joint_matrices[i];
                if !relative_eq!(expected, actual) {
                    println!("Matrix {} is incorrect", i);
                    println!("Actual:");
                    println!("{}", serde_json::to_string_pretty(&actual).unwrap());
                    println!("Expected:");
                    println!("{}", serde_json::to_string_pretty(&expected).unwrap());
                    std::fs::File::create("matrix_failed.json")
                        .unwrap()
                        .write_all(&serde_json::to_vec_pretty(&joint_matrices[..]).unwrap())
                        .unwrap();
                    return false;
                }
            }
            called += 1;
        }
        assert_ne!(called, 0);

        true
    }
}
