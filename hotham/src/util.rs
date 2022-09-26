#![allow(dead_code)]
#![allow(deprecated)]

use anyhow::Result;
use nalgebra::{
    Isometry, Isometry3, Matrix3, Matrix4, Quaternion, Rotation3, Translation3, Unit,
    UnitQuaternion, Vector3,
};
use openxr::{Posef, Quaternionf, SpaceLocation, SpaceLocationFlags, Vector3f, ViewStateFlags};
use std::{ffi::CStr, os::raw::c_char, str::Utf8Error};

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
    cstr.to_str()
}

#[cfg(test)]
use hecs::World;

#[cfg(test)]
use crate::{
    asset_importer::{add_model_to_world, load_models_from_glb},
    contexts::{PhysicsContext, RenderContext, VulkanContext},
};

/// Convenience function to get a world with hands
#[cfg(test)]
pub fn get_world_with_hands(
    vulkan_context: &VulkanContext,
    render_context: &mut RenderContext,
    physics_context: &mut PhysicsContext,
) -> World {
    use crate::components::{LocalTransform, Skin};

    let data: Vec<&[u8]> = vec![
        include_bytes!("../../test_assets/left_hand.glb"),
        include_bytes!("../../test_assets/right_hand.glb"),
    ];
    let models =
        load_models_from_glb(&data, vulkan_context, render_context, physics_context).unwrap();

    let mut world = World::new();

    // Add two hands
    let left_hand =
        add_model_to_world("Left Hand", &models, &mut world, physics_context, None).unwrap();
    {
        let mut local_transform = world.get_mut::<LocalTransform>(left_hand).unwrap();
        local_transform.translation = [-0.2, 1.4, 0.0].into();
    }

    let right_hand =
        add_model_to_world("Right Hand", &models, &mut world, physics_context, None).unwrap();
    {
        let mut local_transform = world.get_mut::<LocalTransform>(right_hand).unwrap();
        local_transform.translation = [0.2, 1.4, 0.0].into();
    }

    // Sanity check
    {
        let mut query = world.query::<&Skin>();
        assert_eq!(query.iter().len(), 2);
    }

    world
}

/// Convert a `Posef` from OpenXR into a `nalgebra::Isometry3`
#[inline]
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

/// Convert a `nalgebra::Isometry3` into a `Posef` from OpenXR
#[inline]
pub fn isometry_to_posef(isometry: Isometry3<f32>) -> Posef {
    Posef {
        orientation: Quaternionf {
            x: isometry.rotation.i,
            y: isometry.rotation.j,
            z: isometry.rotation.k,
            w: isometry.rotation.w,
        },
        position: Vector3f {
            x: isometry.translation.vector.x,
            y: isometry.translation.vector.y,
            z: isometry.translation.vector.z,
        },
    }
}

/// Convert a `Matrix4` into a `nalgebra::Isometry3`
#[inline]
pub fn matrix_to_isometry(m: Matrix4<f32>) -> Isometry3<f32> {
    let translation = m.column(3).xyz();
    let m: Matrix3<f32> = m.fixed_slice::<3, 3>(0, 0).into();
    let rotation = Rotation3::from_matrix(&m);
    Isometry3::from_parts(translation.into(), rotation.into())
}

#[cfg(test)]
use crate::rendering::legacy_buffer::Buffer;
#[cfg(test)]
use ash::vk;
#[cfg(test)]
use std::marker::PhantomData;

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

#[cfg(test)]
pub fn test_buffer<T>() -> Buffer<T> {
    Buffer {
        handle: vk::Buffer::null(),
        device_memory: vk::DeviceMemory::null(),
        _phantom: PhantomData,
        size: 0,
        device_memory_size: 0,
        usage: vk::BufferUsageFlags::empty(),
    }
}

/// Check to see if the current XrSpace is valid
pub fn is_space_valid(space: &SpaceLocation) -> bool {
    space
        .location_flags
        .contains(SpaceLocationFlags::POSITION_VALID)
        && space
            .location_flags
            .contains(SpaceLocationFlags::ORIENTATION_VALID)
}

/// Check to see if the current Xr View is valid
pub fn is_view_valid(view_flags: &ViewStateFlags) -> bool {
    view_flags.contains(ViewStateFlags::POSITION_VALID)
        && view_flags.contains(ViewStateFlags::ORIENTATION_VALID)
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
