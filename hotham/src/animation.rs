use std::{cell::RefCell, rc::Rc};

use anyhow::{anyhow, Result};
use cgmath::{vec3, InnerSpace, Quaternion, Vector3, VectorSpace};
use gltf::animation::util::ReadOutputs;

use crate::node::Node;

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
        buffers: &Vec<&[u8]>,
        nodes: &[&Rc<RefCell<Node>>],
    ) -> Result<()> {
        let mut channels = Vec::new();
        let mut start_time = 0.0;
        let mut end_time = 0.0;
        let mut first_target_node = None;

        for channel in animation.channels() {
            let reader = channel.reader(|buffer| Some(&buffers[buffer.index()]));

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
                        values.push(Quaternion::new(t[3], t[0], t[1], t[2])); // gltf gives us a quaternion in [x, y, z, w] but we need [w, x, y, z]
                    }
                    outputs = AnimationOutputs::Rotations(values);
                }
                _ => panic!("Invalid data!"),
            }

            let sampler = AnimationSampler { inputs, outputs };
            let target_node_index = channel.target().node().index();
            let target_node = find_node(nodes, target_node_index)?;

            if first_target_node.is_none() {
                first_target_node.replace(target_node.clone());
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

        let parent_node = first_target_node
            .unwrap()
            .borrow()
            .get_root_node()
            .ok_or_else(|| anyhow!("Unable to find parent node"))?;

        println!(
            "Found parent node {} for animation!",
            (*parent_node).borrow().index,
        );
        let animation = Rc::new(RefCell::new(animation));
        (*parent_node).borrow_mut().animations.push(animation);

        Ok(())
    }

    pub(crate) fn update_to_percentage(&mut self, percentage: f32) -> () {
        self.current_time = self.end_time * percentage;
        self.update_channels()
    }

    pub(crate) fn update(&mut self, delta_time: f32) -> () {
        self.current_time += delta_time;
        if self.current_time >= self.end_time {
            self.current_time -= self.end_time;
        }

        self.update_channels()
    }

    fn update_channels(&self) -> () {
        let current_time = &self.current_time;
        for channel in &self.channels {
            let sampler = &channel.sampler;
            for i in 0..sampler.inputs.len() {
                let lower_index = i;
                let higher_index = i + 1;

                let lower = sampler.inputs.get(lower_index);
                let higher = sampler.inputs.get(higher_index);

                if lower.is_none() || higher.is_none() {
                    continue;
                }

                let lower = lower.unwrap();
                let higher = higher.unwrap();
                if current_time >= lower && current_time <= higher {
                    // Do this in a block so that we drop the mutable reference to target_node
                    {
                        let mut target_node = channel.target_node.borrow_mut();
                        let interpolation = (current_time - lower) / (higher - lower);
                        match &sampler.outputs {
                            AnimationOutputs::Translations(t) => {
                                let lower = t.get(lower_index).unwrap();
                                let higher = t.get(higher_index).unwrap();
                                let interpolated = lower.lerp(*higher, interpolation);
                                target_node.translation = interpolated;
                            }
                            AnimationOutputs::Rotations(r) => {
                                let lower = r.get(lower_index).unwrap();
                                let higher = r.get(higher_index).unwrap();
                                let interpolated = lower.slerp(*higher, interpolation).normalize();
                                target_node.rotation = interpolated;
                            }
                            AnimationOutputs::Scales(s) => {
                                let lower = s.get(lower_index).unwrap();
                                let higher = s.get(higher_index).unwrap();
                                let interpolated = lower.lerp(*higher, interpolation);
                                target_node.scale = interpolated;
                            }
                        }
                    }
                }
            }
        }
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
        gltf_loader, renderer::create_descriptor_set_layouts, vulkan_context::VulkanContext,
    };
    use ash::vk;

    #[test]
    pub fn animation_test_hand() {
        let (document, buffers, _) = gltf::import("../test_assets/hand.gltf").unwrap();
        let vulkan_context = VulkanContext::testing().unwrap();
        let set_layouts = create_descriptor_set_layouts(&vulkan_context).unwrap();
        let ubo_buffer = vk::Buffer::null();
        let buffers = buffers.iter().map(|b| b.0.as_slice()).collect();

        let gltf_bytes = document.into_json().to_vec().unwrap();
        let nodes = gltf_loader::load_gltf_nodes(
            &gltf_bytes,
            &buffers,
            &vulkan_context,
            &set_layouts,
            ubo_buffer,
        )
        .unwrap();

        let hand = nodes.get("Hand").unwrap();
        {
            let mut hand = hand.borrow_mut();
            hand.active_animation_index.replace(0);
        }

        let hand = hand.borrow();
        let before = hand.find(2).unwrap().borrow().get_node_matrix();

        let delta_time = 0.5;
        hand.update_animation(delta_time, &vulkan_context).unwrap();

        let after = hand.find(2).unwrap().borrow().get_node_matrix();
        assert_ne!(before, after);
    }

    #[test]
    pub fn animation_test_simple() {
        let (document, buffers, _) = gltf::import("../test_assets/animation_test.gltf").unwrap();
        let buffers = buffers.iter().map(|b| b.0.as_slice()).collect();
        let vulkan_context = VulkanContext::testing().unwrap();
        let set_layouts = create_descriptor_set_layouts(&vulkan_context).unwrap();
        let ubo_buffer = vk::Buffer::null();

        let gltf_bytes = document.into_json().to_vec().unwrap();
        let nodes = gltf_loader::load_gltf_nodes(
            &gltf_bytes,
            &buffers,
            &vulkan_context,
            &set_layouts,
            ubo_buffer,
        )
        .unwrap();

        let test = nodes.get("Test").unwrap();
        {
            let mut hand = test.borrow_mut();
            hand.active_animation_index.replace(0);
        }

        let test = test.borrow();
        let before = test.find(2).unwrap().borrow().get_node_matrix();

        let delta_time = 0.1;
        test.update_animation(delta_time, &vulkan_context).unwrap();

        let after = test.find(2).unwrap().borrow().get_node_matrix();
        assert_ne!(before, after);
    }
}
