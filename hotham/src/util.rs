#![allow(dead_code)]

use anyhow::Result;
use legion::{Entity, World};
use nalgebra::{
    vector, Isometry, Isometry3, Quaternion, Translation3, Unit, UnitQuaternion, Vector3,
};
use openxr::Posef;
use std::{ffi::CStr, os::raw::c_char, str::Utf8Error};

use crate::{
    add_model_to_world,
    components::Transform,
    gltf_loader::load_models_from_glb,
    resources::{render_context::create_descriptor_set_layouts, VulkanContext},
};

pub(crate) unsafe fn get_raw_strings(strings: Vec<&str>) -> Vec<*const c_char> {
    strings
        .iter()
        .map(|s| CStr::from_bytes_with_nul_unchecked(s.as_bytes()).as_ptr())
        .collect::<Vec<_>>()
}

pub(crate) unsafe fn parse_raw_strings(raw_strings: &[*const c_char]) -> Vec<&str> {
    raw_strings
        .iter()
        .filter_map(|s| parse_raw_string(*s).ok())
        .collect::<Vec<_>>()
}

pub(crate) unsafe fn parse_raw_string(
    raw_string: *const c_char,
) -> Result<&'static str, Utf8Error> {
    let cstr = CStr::from_ptr(raw_string);
    return cstr.to_str();
}

pub fn get_world_with_hands() -> World {
    let vulkan_context = VulkanContext::testing().unwrap();
    let set_layouts = create_descriptor_set_layouts(&vulkan_context).unwrap();

    let data: Vec<&[u8]> = vec![
        include_bytes!("../../hotham-asteroid/assets/left_hand.glb"),
        include_bytes!("../../hotham-asteroid/assets/right_hand.glb"),
    ];
    let models = load_models_from_glb(&data, &vulkan_context, &set_layouts).unwrap();

    let mut world = World::default();

    // Add two hands
    let left_hand = add_model_to_world("Left Hand", &models, &mut world, None).unwrap();
    {
        let mut left_hand_entity = world.entry(left_hand).unwrap();
        let transform = left_hand_entity.get_component_mut::<Transform>().unwrap();
        transform.translation = vector![-0.2, 1.4, 0.0];
    }

    let right_hand = add_model_to_world("Right Hand", &models, &mut world, None).unwrap();
    {
        let mut right_hand_entity = world.entry(right_hand).unwrap();
        let transform = right_hand_entity.get_component_mut::<Transform>().unwrap();
        transform.translation = vector![0.2, 1.4, 0.0];
    }

    world
}

pub fn entity_to_u64(entity: Entity) -> u64 {
    unsafe { std::mem::transmute(entity) }
}
pub fn u64_to_entity(entity: u64) -> Entity {
    unsafe { std::mem::transmute(entity) }
}

pub fn posef_to_isometry(pose: Posef) -> Isometry3<f32> {
    let translation: Vector3<f32> = mint::Vector3::from(pose.position).into();
    let translation: Translation3<f32> = Translation3::from(translation);
    let rotation: Quaternion<f32> = mint::Quaternion::from(pose.orientation).into();
    let rotation: UnitQuaternion<f32> = Unit::new_normalize(rotation);
    Isometry {
        rotation,
        translation,
    }
}

#[cfg(test)]
use crate::buffer::Buffer;

#[cfg(test)]
pub unsafe fn get_from_device_memory<'a, T: Sized>(
    vulkan_context: &VulkanContext,
    buffer: &'a Buffer<T>,
) -> &'a [T] {
    let memory = vulkan_context
        .device
        .map_memory(
            buffer.device_memory,
            0,
            ash::vk::WHOLE_SIZE,
            ash::vk::MemoryMapFlags::empty(),
        )
        .unwrap();
    std::slice::from_raw_parts(std::mem::transmute(memory), buffer.size as _)
}

#[cfg(target_os = "android")]
pub(crate) fn get_asset_from_path(path: &str) -> Result<Vec<u8>> {
    use anyhow::anyhow;
    let native_activity = ndk_glue::native_activity();

    let asset_manager = native_activity.asset_manager();
    let path_with_nul = format!("{}\0", path);
    let path = unsafe { CStr::from_bytes_with_nul_unchecked(path_with_nul.as_bytes()) };

    let mut asset = asset_manager
        .open(path)
        .ok_or(anyhow!("Can't open: {:?}", path))?;

    Ok(asset.get_buffer()?.to_vec())
}
