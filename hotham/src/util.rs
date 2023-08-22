#![allow(dead_code)]
#![allow(deprecated)]

use anyhow::Result;
use glam::{Affine3A, Quat, Vec3};
use openxr::{Posef, SpaceLocation, SpaceLocationFlags, ViewStateFlags};
use rapier3d::na::Vector3;
use std::{ffi::CStr, os::raw::c_char, str::Utf8Error, sync::Arc, time::Instant};

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

// shout out to wgpu to for this:
/// Creates a new vec with a copy of the same bytes.
pub fn u8_to_u32(asset_data: Arc<Vec<u8>>) -> Vec<u32> {
    let mut words = vec![0u32; asset_data.len() / std::mem::size_of::<u32>()];
    unsafe {
        std::ptr::copy_nonoverlapping(
            asset_data.as_ptr(),
            words.as_mut_ptr() as *mut u8,
            asset_data.len(),
        );
    }

    words
}

#[cfg(test)]
use {hecs::World, std::env};

#[cfg(test)]
use crate::{
    asset_importer::{add_model_to_world, load_models_from_glb},
    contexts::{RenderContext, VulkanContext},
};

/// Convenience function to get a world with hands
#[cfg(test)]
pub fn get_world_with_hands(
    vulkan_context: &VulkanContext,
    render_context: &mut RenderContext,
) -> World {
    use crate::components::{GlobalTransform, Skin};

    let data: Vec<&[u8]> = vec![
        include_bytes!("../../test_assets/left_hand.glb"),
        include_bytes!("../../test_assets/right_hand.glb"),
    ];
    let models = load_models_from_glb(&data, vulkan_context, render_context).unwrap();

    let mut world = World::new();

    // Add two hands
    let left_hand = add_model_to_world("Left Hand", &models, &mut world, None).unwrap();
    {
        let mut global_transform = world.get::<&mut GlobalTransform>(left_hand).unwrap();
        global_transform.0.translation = [-0.2, 1.4, 0.0].into();
    }

    let right_hand = add_model_to_world("Right Hand", &models, &mut world, None).unwrap();
    {
        let mut global_transform = world.get::<&mut GlobalTransform>(right_hand).unwrap();
        global_transform.0.translation = [0.2, 1.4, 0.0].into();
    }

    // Sanity check
    {
        let mut query = world.query::<&Skin>();
        assert_eq!(query.iter().len(), 2);
    }

    world
}

#[inline]
/// Convert a `Posef` from OpenXR into an Affine3
pub fn affine_from_posef(pose: Posef) -> Affine3A {
    let translation: Vec3 = mint::Vector3::from(pose.position).into();
    let rotation: Quat = mint::Quaternion::from(pose.orientation).into();

    Affine3A::from_rotation_translation(rotation, translation)
}

#[inline]
/// Convert a [`glam::Affine3A`] into a [`openxr::Posef`]
pub fn posef_from_affine(transform: Affine3A) -> Posef {
    let (_, rotation, translation) = transform.to_scale_rotation_translation();
    Posef {
        orientation: mint::Quaternion::from(rotation).into(),
        position: mint::Vector3::from(translation).into(),
    }
}

#[inline]
/// Convert a [`glam::Affine3A`] into a [`rapier3d::na::Isometry3`]
pub fn isometry_from_affine(a: &glam::f32::Affine3A) -> rapier3d::na::Isometry3<f32> {
    use rapier3d::na;
    let (_, r, t) = a.to_scale_rotation_translation();
    let translation = na::Translation3::new(t.x, t.y, t.z);

    let rotation: na::UnitQuaternion<f32> =
        na::UnitQuaternion::new_unchecked([r.x, r.y, r.z, r.w].into());

    na::Isometry3::from_parts(translation, rotation)
}

#[inline]
/// Decompose a [`rapier3d::na::Isometry3`] into its rotation and translation components
pub fn decompose_isometry(i: &rapier3d::na::Isometry3<f32>) -> (glam::Quat, glam::Vec3) {
    (
        glam::Quat::from_array(i.rotation.quaternion().coords.data.0[0]),
        mint::Vector3::from(i.translation.vector.data.0[0]).into(),
    )
}

#[inline]
/// Convert a [`glam::Vec3`] into a [`rapier3d::na::Vector3`]
pub fn na_vector_from_glam(v: Vec3) -> Vector3<f32> {
    [v.x, v.y, v.z].into()
}

#[inline]
/// Convert a [`glam::Vec3`] into a [`rapier3d::na::Vector3`]
pub fn glam_vec_from_na(v: &Vector3<f32>) -> glam::Vec3 {
    [v.x, v.y, v.z].into()
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

#[cfg(test)]
pub(crate) unsafe fn save_image_to_disk(
    vulkan_context: &VulkanContext,
    image: crate::rendering::image::Image,
    name: &str,
) -> Result<(), String> {
    use crate::rendering::buffer::Buffer;
    use image::{codecs::jpeg::JpegEncoder, DynamicImage, RgbaImage};

    let resolution = image.extent;
    let size = (resolution.height * resolution.width * 4) as usize;
    let mut buffer = Buffer::new(vulkan_context, vk::BufferUsageFlags::TRANSFER_DST, size);

    vulkan_context.device.device_wait_idle().unwrap();

    vulkan_context.transition_image_layout(
        image.handle,
        vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
        1,
        1,
    );
    vulkan_context.copy_image_to_buffer(
        &image,
        vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
        buffer.buffer,
    );

    // We have to set the buffer's length manually as `copy_image_to_buffer` doesn't.
    buffer.len = size;

    vulkan_context.device.device_wait_idle().unwrap();
    let image_bytes = buffer.as_slice().to_vec();
    assert_eq!(image_bytes.len(), size);
    let image_from_vulkan = DynamicImage::ImageRgba8(
        RgbaImage::from_raw(resolution.width, resolution.height, image_bytes).unwrap(),
    );
    let known_good_path = format!("../test_assets/render_{name}_known_good.jpg");
    if env::var("UPDATE_IMAGES").map_or(false, |s| {
        s.eq_ignore_ascii_case("true")
            || s.eq_ignore_ascii_case("t")
            || s.eq_ignore_ascii_case("yes")
            || s.eq_ignore_ascii_case("y")
            || s == "1"
    }) {
        let output_path = std::path::Path::new(&known_good_path);
        let mut file = std::fs::File::create(output_path).unwrap();
        let mut jpeg_encoder = JpegEncoder::new(&mut file);
        jpeg_encoder.encode_image(&image_from_vulkan).unwrap();
    }
    let output_path = format!("../test_assets/render_{name}.jpg");
    {
        let output_path = std::path::Path::new(&output_path);
        let mut file = std::fs::File::create(output_path).unwrap();
        let mut jpeg_encoder = JpegEncoder::new(&mut file);
        jpeg_encoder.encode_image(&image_from_vulkan).unwrap();
    }
    let output_hash = hash_file(&output_path);
    let known_good_hash = hash_file(&known_good_path);

    if output_hash.is_err() {
        return Err(format!("Failed to hash output image: {name}"));
    }
    if known_good_hash.is_err() {
        return Err(format!("Failed to hash known good image: {name}"));
    }
    if output_hash != known_good_hash {
        return Err(format!("Bad render: {name}"));
    }
    Ok(())
}

fn hash_file(file_path: &str) -> anyhow::Result<u64, ()> {
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();

    let bytes = match std::fs::read(file_path) {
        Ok(it) => it,
        Err(_) => return Err(()),
    };
    bytes.iter().for_each(|b| hasher.write_u8(*b));
    Ok(hasher.finish())
}

#[cfg(all(test, not(any(target_os = "macos", target_os = "ios"))))]
use renderdoc::RenderDoc;

#[cfg(all(test, not(any(target_os = "macos", target_os = "ios"))))]
pub(crate) fn begin_renderdoc() -> Result<RenderDoc<renderdoc::V141>, renderdoc::Error> {
    let mut renderdoc = RenderDoc::<renderdoc::V141>::new()?;
    renderdoc.start_frame_capture(std::ptr::null(), std::ptr::null());
    Ok(renderdoc)
}

#[cfg(all(test, not(any(target_os = "macos", target_os = "ios"))))]
pub(crate) fn end_renderdoc(renderdoc: &mut RenderDoc<renderdoc::V141>) {
    renderdoc.end_frame_capture(std::ptr::null(), std::ptr::null());
}

/// Interpolate between two affine transforms
pub fn lerp_slerp(a: &Affine3A, b: &Affine3A, s: f32) -> Affine3A {
    let (a_scale, a_rotation, a_translation) = a.to_scale_rotation_translation();
    let (b_scale, b_rotation, b_translation) = b.to_scale_rotation_translation();

    Affine3A::from_scale_rotation_translation(
        a_scale.lerp(b_scale, s),
        a_rotation.slerp(b_rotation, s),
        a_translation.lerp(b_translation, s),
    )
}

#[derive(Debug)]
/// A timer to track performance
pub struct PerformanceTimer {
    name: String,
    frame_start: Instant,
    timings: Vec<usize>,
    last_update: Instant,
}

impl PerformanceTimer {
    /// Start tracking
    pub fn start(&mut self) {
        self.frame_start = Instant::now();
    }

    /// Stop tracking
    pub fn end(&mut self) {
        let now = Instant::now();
        let tic_time = now - self.frame_start;
        self.timings.push(tic_time.as_millis() as usize);

        if (now - self.last_update).as_secs_f32() >= 1.0 {
            let average = self.timings.iter().sum::<usize>() / self.timings.len();
            let name = &self.name;
            if average > 0 {
                println!("[HOTHAM_PERF] Warning: {name} took {average}ms, you might be doing too much work on the CPU");
            }
            self.last_update = now;
            self.timings.clear();
        }
    }

    /// Create a new performance timer
    pub fn new(name: &'static str) -> Self {
        Self {
            name: name.to_string(),
            frame_start: Instant::now(),
            last_update: Instant::now(),
            timings: Default::default(),
        }
    }
}
