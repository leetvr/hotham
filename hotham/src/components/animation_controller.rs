use std::collections::HashMap;

use gltf::animation::util::ReadOutputs;
use itertools::Itertools;

use crate::{asset_importer::ImportContext, components::AnimationTarget};
use glam::Quat;

#[derive(Debug, Clone, PartialEq, Default)]

/// Component that controls how an `AnimationTarget` should be animated.
/// Added by `gltf_loader` to the root node if its children contain animation data.
pub struct AnimationController {
    /// The amount to blend from
    pub blend_from: usize,
    /// The amount to blend to
    pub blend_to: usize,
    /// The total blend amount
    pub blend_amount: f32,
    /// The targets to apply this animation to
    pub targets: Vec<AnimationTarget>,
}

impl AnimationController {
    pub(crate) fn load(
        animations: gltf::iter::Animations,
        import_context: &mut ImportContext,
    ) -> AnimationController {
        let node_entity_map = &import_context.node_entity_map;
        let buffer = &import_context.buffer;

        let mut targets = HashMap::new();

        for channel in animations.flat_map(|a| a.channels()) {
            let target = *node_entity_map
                .get(&channel.target().node().index())
                .unwrap();

            let animation_target = targets.entry(target).or_insert(AnimationTarget {
                target,
                rotations: Vec::new(),
                scales: Vec::new(),
                translations: Vec::new(),
            });

            let reader = channel.reader(|_| Some(buffer));
            match reader.read_outputs() {
                Some(ReadOutputs::Translations(translation_data)) => {
                    for t in translation_data {
                        animation_target
                            .translations
                            .push([t[0], t[1], t[2]].into());
                    }
                }
                Some(ReadOutputs::Rotations(rotation_data)) => {
                    for r in rotation_data.into_f32() {
                        animation_target
                            .rotations
                            .push(Quat::from_xyzw(r[0], r[1], r[2], r[3]));
                    }
                }
                Some(ReadOutputs::Scales(scale_data)) => {
                    for s in scale_data {
                        animation_target.scales.push([s[0], s[1], s[2]].into());
                    }
                }
                _ => {}
            }
        }

        AnimationController {
            blend_from: 0,
            blend_to: 1,
            blend_amount: 0.,
            targets: targets.drain().map(|n| n.1).collect_vec(),
        }
    }
}
