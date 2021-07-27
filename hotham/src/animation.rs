use std::{cell::RefCell, rc::Rc};

use anyhow::{anyhow, Result};
use cgmath::{vec3, InnerSpace, Quaternion, Vector3, VectorSpace};
use gltf::animation::util::ReadOutputs;

use crate::{node::Node, vulkan_context::VulkanContext};
use itertools::Itertools;

#[derive(Debug, Clone)]
pub struct Animation {
    channels: Vec<AnimationChannel>,
    start_time: f32,
    end_time: f32,
    current_time: f32,
}

impl Animation {
    pub(crate) fn load(
        animation: &gltf::Animation,
        blob: &[u8],
        nodes: &[&Rc<RefCell<Node>>],
    ) -> Result<()> {
        let mut channels = Vec::new();
        let mut start_time = 0.0;
        let mut end_time = 0.0;
        let mut first_node_index = None;

        for channel in animation.channels() {
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
            let target_node_index = channel.target().node().index();
            let target_node = find_node(nodes, target_node_index)?;

            if first_node_index.is_none() {
                first_node_index = Some(target_node_index);
            }

            channels.push(AnimationChannel {
                sampler,
                target_node,
            })
        }

        let animation = Animation {
            channels,
            start_time,
            end_time,
            current_time: 0.0,
        };

        let first_node_index = first_node_index.unwrap();
        let parent_node = find_parent_node(nodes, first_node_index).ok_or_else(|| {
            anyhow!(
                "Unable to find parent node with first index: {}",
                first_node_index
            )
        })?;
        println!(
            "Found parent node {} for first_node_index {}",
            (*parent_node).borrow().index,
            first_node_index
        );
        let animation = Rc::new(RefCell::new(animation));
        (*parent_node).borrow_mut().animations.push(animation);

        Ok(())
    }

    pub(crate) fn update(&mut self, delta_time: f32, vulkan_context: &VulkanContext) -> Result<()> {
        self.current_time += delta_time;
        if self.current_time >= self.end_time {
            self.current_time -= self.end_time;
        }
        let current_time = &self.current_time;

        for channel in &self.channels {
            let sampler = &channel.sampler;
            for mut chunk in &sampler.inputs.iter().enumerate().chunks(2) {
                let lower = chunk.next();
                let higher = chunk.next();

                if lower.is_none() || higher.is_none() {
                    continue;
                }

                let (lower_index, lower) = lower.unwrap();
                let (higher_index, higher) = higher.unwrap();
                if current_time >= lower && current_time <= higher {
                    let mut target_node = channel.target_node.borrow_mut();
                    let interpolation = (current_time - lower) / (higher - lower);
                    match &sampler.outputs {
                        AnimationOutputs::Translations(t) => {
                            let lower = t.get(lower_index).unwrap();
                            let higher = t.get(higher_index).unwrap();
                            let interpolated = lower.lerp(*higher, interpolation);
                            target_node.translation = interpolated;
                            println!(
                                "{} translation is now {:?}",
                                target_node.index, interpolated
                            );
                        }
                        AnimationOutputs::Rotations(r) => {
                            let lower = r.get(lower_index).unwrap();
                            let higher = r.get(higher_index).unwrap();
                            let interpolated = lower.slerp(*higher, interpolation).normalize();
                            target_node.rotation = interpolated;
                            println!("{} rotation is now {:?}", target_node.index, interpolated);
                        }
                        AnimationOutputs::Scales(s) => {
                            let lower = s.get(lower_index).unwrap();
                            let higher = s.get(higher_index).unwrap();
                            let interpolated = lower.lerp(*higher, interpolation);
                            target_node.scale = interpolated;
                            println!("{} scale is now {:?}", target_node.index, interpolated);
                        }
                    }
                    target_node.update_joints(vulkan_context)?;
                }
            }
        }

        Ok(())
    }
}

fn find_node(nodes: &[&Rc<RefCell<Node>>], index: usize) -> Result<Rc<RefCell<Node>>> {
    for n in nodes {
        if let Some(found) = (***n).borrow().find(index) {
            return Ok(found);
        }
    }

    Err(anyhow!("Unable to find node with index: {}", index))
}

fn find_parent_node(nodes: &[&Rc<RefCell<Node>>], first_index: usize) -> Option<Rc<RefCell<Node>>> {
    // What we want to do is find the node that has a skin whose skeleton root is the parent node
    // This is all based on the fact that the skeleton root is the first node in the hierarchy
    for node in nodes {
        let n = (***node).borrow();
        let skin = n.skin.as_ref();
        if let Some(skin) = skin {
            if skin.skeleton_root_index == first_index {
                return Some(Rc::clone(node));
            }
        }

        let children = &n.children;
        if children.len() == 0 {
            return None;
        }

        let children = children.iter().collect::<Vec<_>>();
        if let Some(found) = find_parent_node(&children, first_index) {
            return Some(found);
        }
    }

    None
}

#[derive(Debug, Clone)]
pub(crate) struct AnimationSampler {
    inputs: Vec<f32>,
    outputs: AnimationOutputs,
}

#[derive(Debug, Clone)]
pub(crate) struct AnimationChannel {
    sampler: AnimationSampler,
    target_node: Rc<RefCell<Node>>,
}

#[derive(Debug, Clone)]
pub(crate) enum AnimationOutputs {
    Translations(Vec<Vector3<f32>>),
    Rotations(Vec<Quaternion<f32>>),
    Scales(Vec<Vector3<f32>>),
}

#[cfg(test)]
mod tests {
    use crate::{
        gltf_loader, renderer::create_descriptor_set_layout, vulkan_context::VulkanContext,
    };
    use ash::vk;

    #[test]
    pub fn animation_test() {
        let (document, buffers, _) = gltf::import("../test_assets/hand.gltf").unwrap();
        let vulkan_context = VulkanContext::testing().unwrap();
        let set_layout = create_descriptor_set_layout(&vulkan_context).unwrap();
        let ubo_buffer = vk::Buffer::null();

        let gltf_bytes = document.into_json().to_vec().unwrap();
        let data_bytes = &buffers[0];
        let nodes = gltf_loader::load_gltf_nodes(
            &gltf_bytes,
            data_bytes,
            &vulkan_context,
            &[set_layout],
            ubo_buffer,
        )
        .unwrap();

        let hand = nodes.get("Hand").unwrap().borrow();
        let hand_root = hand.children.first().unwrap();
        let hand_root = hand_root.borrow();
        let before = hand.find(2).unwrap().borrow().get_node_matrix();

        {
            let animation = hand_root.animations.first().unwrap();
            let delta_time = 0.5;
            animation
                .borrow_mut()
                .update(delta_time, &vulkan_context)
                .unwrap();
        }

        let after = hand.find(2).unwrap().borrow().get_node_matrix();
        assert_ne!(before, after);
    }
}
