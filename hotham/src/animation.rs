use cgmath::{vec3, Quaternion, Vector3};
use gltf::{animation::util::ReadOutputs, Document};

use crate::node::Node;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Animation {
    channels: Vec<AnimationChannel>,
    start_time: f32,
    end_time: f32,
    current_time: f32,
}

impl Animation {
    pub(crate) fn load(document: &Document, blob: &[u8]) -> Vec<Self> {
        document
            .animations()
            .map(|a| Animation::new(&a, &blob))
            .collect()
    }

    fn new(animation: &gltf::Animation, blob: &[u8]) -> Animation {
        let mut channels = Vec::new();
        let mut start_time = 0.0;
        let mut end_time = 0.0;

        for channel in animation.channels() {
            assert_eq!(blob.len(), 100);
            let reader = channel.reader(|_| Some(&blob));

            let mut inputs = Vec::new();
            let outputs;

            if let Some(iter) = reader.read_inputs() {
                for input in iter {
                    if input < start_time {
                        start_time = input;
                    }
                    if input > end_time {
                        end_time = input;
                    }
                    inputs.push(input);
                }
            }

            match reader.read_outputs() {
                Some(ReadOutputs::Translations(translations)) => {
                    let mut values = Vec::new();
                    for t in translations {
                        values.push(vec3(t[0], t[1], t[2]));
                    }
                    outputs = AnimationOutputs::Translations(values);
                }
                Some(ReadOutputs::Scales(scales)) => {
                    let mut values = Vec::new();
                    for t in scales {
                        values.push(vec3(t[0], t[1], t[2]));
                    }
                    outputs = AnimationOutputs::Scales(values);
                }
                Some(ReadOutputs::Rotations(rotations)) => {
                    let mut values = Vec::new();
                    for t in rotations.into_f32() {
                        values.push(Quaternion::new(t[0], t[1], t[2], t[3]));
                    }
                    outputs = AnimationOutputs::Rotations(values);
                }
                _ => panic!("Invalid data!"),
            }

            let sampler = AnimationSampler { inputs, outputs };

            channels.push(AnimationChannel {
                sampler,
                target_node_index: channel.target().node().index(),
            })
        }

        Animation {
            channels,
            start_time,
            end_time,
            current_time: 0.0,
        }
    }

    pub fn update(delta_time: f32, root_node: &mut Node) {}
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AnimationSampler {
    inputs: Vec<f32>,
    outputs: AnimationOutputs,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AnimationChannel {
    sampler: AnimationSampler,
    target_node_index: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AnimationOutputs {
    Translations(Vec<Vector3<f32>>),
    Rotations(Vec<Quaternion<f32>>),
    Scales(Vec<Vector3<f32>>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn animation_test() {
        let (document, buffers, _) = gltf::import("../test_assets/animation_test.gltf").unwrap();
        let data = &buffers[1];
        assert_eq!(data.len(), 100);
        let mut animations = Animation::load(&document, data);
        assert_eq!(animations.len(), 1);
        let mut animation = animations.pop().unwrap();
        let delta_time = 0.5;
        // let mut nodes = todo!();
        // animation.update(delta_time, &mut nodes);

        // Load test animation from glTF
        // Get interpolated value
        // Test
    }
}
