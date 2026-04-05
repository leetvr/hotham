use std::collections::HashMap;

use gltf::animation::util::ReadOutputs;
use itertools::Itertools;

use crate::{
    asset_importer::ImportContext,
    components::{animation_target::AnimationOutputs, AnimationTarget},
    contexts::physics_context::DELTA_TIME,
};
use glam::Quat;

#[derive(Debug, Clone, PartialEq, Default)]

/// Component that controls how an `AnimationTarget` should be animated.
/// Added by `gltf_loader` to the root node if its children contain animation data.
pub struct AnimationController {
    pub animations: HashMap<String, AnimationState>,
    pub current_animation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AnimationState {
    /// The amount of time that has elapsed
    pub elapsed: f32,
    /// The total duration of this animation
    pub duration: f32,
    /// The targets to apply this animation to
    pub targets: Vec<AnimationTarget>,
}

impl AnimationController {
    pub fn current_animation(&self) -> Option<&AnimationState> {
        let current_animation = self.current_animation.as_ref()?;
        self.animations.get(current_animation)
    }

    pub fn current_animation_mut(&mut self) -> Option<&mut AnimationState> {
        let current_animation = self.current_animation.as_ref()?;
        self.animations.get_mut(current_animation)
    }

    pub fn set_current_animation(&mut self, new_current_animation: impl Into<String>) {
        self.current_animation.replace(new_current_animation.into());
        let Some(current) = self.current_animation_mut() else {
            return;
        };

        current.elapsed = 0.;
    }

    pub fn advance_animation(&mut self) {
        let Some(current) = self.current_animation_mut() else {
            return;
        };

        current.elapsed += DELTA_TIME;
    }

    pub(crate) fn load(
        animations: gltf::iter::Animations,
        import_context: &mut ImportContext,
    ) -> AnimationController {
        let node_entity_map = &import_context.node_entity_map;
        let buffer = &import_context.buffer;

        let mut animations_map = HashMap::new();
        let mut current_animation = None;

        for animation in animations {
            if let Some(name) = animation.name() {
                println!("Loading animation #{} ({name})", animation.index());
            } else {
                println!("Loading animation {}", animation.index());
            }

            let mut targets = Vec::new();
            let mut duration = 0.;
            for channel in animation.channels() {
                println!("Loading channel {}", channel.index());
                let target = *node_entity_map
                    .get(&channel.target().node().index())
                    .unwrap();

                let reader = channel.reader(|_| Some(buffer));

                let Some(times) = reader.read_inputs() else {
                    continue;
                };

                let times = times.collect_vec();

                // The duration of the animation is the maximum time
                if let Some(max) = times.iter().max_by(|a, b| a.total_cmp(b)) {
                    if *max > duration {
                        duration = *max;
                    }
                }

                let Some(outputs) = reader.read_outputs() else {
                    continue;
                };

                let outputs = match outputs {
                    ReadOutputs::Translations(translation_data) => {
                        AnimationOutputs::Translation(translation_data.map(|t| t.into()).collect())
                    }
                    ReadOutputs::Rotations(rotation_data) => AnimationOutputs::Rotation(
                        rotation_data
                            .into_f32()
                            .map(|r| glam::quat(r[0], r[1], r[2], r[3]))
                            .collect(),
                    ),
                    ReadOutputs::Scales(scale_data) => {
                        AnimationOutputs::Scales(scale_data.map(|t| t.into()).collect())
                    }
                    // Unsupported
                    ReadOutputs::MorphTargetWeights(_) => {
                        continue;
                    }
                };

                targets.push(AnimationTarget {
                    target,
                    times,
                    outputs,
                });
            }

            let name = animation.name().unwrap_or("").to_string();
            if current_animation.is_none() {
                current_animation = Some(name.clone());
            }

            animations_map.insert(
                name,
                AnimationState {
                    targets,
                    elapsed: 0.,
                    duration,
                },
            );
        }

        AnimationController {
            animations: animations_map,
            current_animation,
        }
    }
}
