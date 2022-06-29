use std::mem::MaybeUninit;

use crate::asset_importer::ImportContext;
use hecs::Entity;
use nalgebra::Matrix4;

pub static NO_SKIN: u32 = std::u32::MAX;

/// Component added to an entity to point to the joints in the node
/// Automatically added by `gltf_loader`
#[derive(Debug, Clone, PartialEq)]
pub struct Skin {
    /// List of joints
    pub joints: Vec<Entity>,
    /// Inverse bind matrices, used to build the final joint matrices for this skin
    pub inverse_bind_matrices: Vec<Matrix4<f32>>,
    /// Index into skin buffer
    pub(crate) id: u32,
}

impl Skin {
    pub(crate) fn load(skin: gltf::Skin, import_context: &mut ImportContext) -> Skin {
        let reader = skin.reader(|_| Some(&import_context.buffer));
        let inverse_bind_matrices = reader
            .read_inverse_bind_matrices()
            .unwrap() //}
            .map(Matrix4::from)
            .collect();

        let joints = skin
            .joints()
            .map(|j| {
                import_context
                    .node_entity_map
                    .get(&j.index())
                    .cloned()
                    .unwrap()
            })
            .collect();

        // Nasty, but safe - this data will be correctly populated when the skin system runs:
        // Having a Skin and not running the skin system is forbidden!
        let id = unsafe {
            import_context
                .render_context
                .resources
                .skins_buffer
                .push(&MaybeUninit::uninit().assume_init())
        };

        Skin {
            joints,
            id,
            inverse_bind_matrices,
        }
    }
}
