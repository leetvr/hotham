use crate::{hotham_error::HothamError, util::get_raw_strings, Result};
use ash::{
    extensions::khr,
    version::{EntryV1_0, InstanceV1_0},
    vk::{self, Handle},
    Device, Entry, Instance,
};
use openxr::{self as xr};
use std::{fmt::Debug, intrinsics::transmute};

#[derive(Clone)]
pub(crate) struct VulkanContext {
    entry: Entry,
    instance: Instance,
    physical_device: vk::PhysicalDevice,
    device: Device,
}

impl VulkanContext {
    pub fn create_from_xr_instance(
        xr_instance: &xr::Instance,
        system: xr::SystemId,
    ) -> Result<Self> {
        let entry = unsafe { Entry::new() }?;
        let get_instance_proc_addr = unsafe { transmute(entry.static_fn().get_instance_proc_addr) };

        let app_info = vk::ApplicationInfo::builder()
            .api_version(vk::make_version(1, 2, 0))
            .build();

        let create_info = vk::InstanceCreateInfo::builder().application_info(&app_info);

        let instance_handle = unsafe {
            xr_instance.create_vulkan_instance(
                system,
                get_instance_proc_addr,
                &create_info as *const _ as *const _,
            )
        }?
        .map_err(vk::Result::from_raw)?;

        let instance = unsafe {
            Instance::load(
                entry.static_fn(),
                vk::Instance::from_raw(instance_handle as _),
            )
        };

        let physical_device = vk::PhysicalDevice::from_raw(
            xr_instance.vulkan_graphics_device(system, instance_handle)? as _,
        );

        let queue_family_index = unsafe {
            instance
                .get_physical_device_queue_family_properties(physical_device)
                .into_iter()
                .enumerate()
                .find_map(|(queue_family_index, info)| {
                    if info.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                        Some(queue_family_index as u32)
                    } else {
                        None
                    }
                })
                .ok_or(HothamError::EmptyListError)?
        };

        let graphics_queue = vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(queue_family_index)
            .queue_priorities(&[1.0])
            .build();
        let queue_create_infos = [graphics_queue];
        let multiview = &mut vk::PhysicalDeviceVulkan11Features {
            multiview: vk::TRUE,
            ..Default::default()
        };

        let create_info = &vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_create_infos)
            .push_next(multiview) as *const _ as *const _;

        let device_handle = unsafe {
            xr_instance.create_vulkan_device(
                system,
                get_instance_proc_addr,
                physical_device.as_raw() as *const _,
                create_info as *const _ as *const _,
            )
        }?
        .map_err(vk::Result::from_raw)?;

        let device =
            unsafe { Device::load(instance.fp_v1_0(), vk::Device::from_raw(device_handle as _)) };

        Ok(Self {
            entry,
            instance,
            device,
            physical_device,
        })
    }
}

impl Debug for VulkanContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VulkanContext")
            .field("entry", &"Vulkan Entry".to_string())
            .field("instance", &"Vulkan Entry".to_string())
            .finish()
    }
}

impl Default for VulkanContext {
    fn default() -> Self {
        let (entry, instance) = unsafe { init_vulkan() }.expect("Unable to initialise Vulkan");
        let physical_device =
            unsafe { get_physical_device(&instance) }.expect("Unable to get physical device");
        let device = unsafe { create_device(&instance, physical_device) }
            .expect("Unable to create logical device");
        Self {
            entry,
            instance,
            physical_device,
            device,
        }
    }
}

unsafe fn create_device(
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
) -> Result<Device> {
    let enabled_extension_names = get_device_extensions();
    let graphics_queue = vk::DeviceQueueCreateInfo::builder()
        .queue_family_index(0)
        .queue_priorities(&[1.0])
        .build();
    let queue_create_infos = [graphics_queue];
    let create_info = vk::DeviceCreateInfo::builder()
        .enabled_extension_names(&enabled_extension_names)
        .queue_create_infos(&queue_create_infos);
    instance
        .create_device(physical_device, &create_info, None)
        .map_err(|e| HothamError::VulkanError { source: e })
}

unsafe fn get_physical_device(instance: &Instance) -> Result<vk::PhysicalDevice> {
    instance
        .enumerate_physical_devices()?
        .pop()
        .ok_or(HothamError::EmptyListError)
}

unsafe fn init_vulkan() -> Result<(Entry, Instance)> {
    let entry = Entry::new()?;

    let app_info = vk::ApplicationInfo::builder()
        .api_version(vk::make_version(1, 2, 0))
        .build();
    let enabled_layer_names = get_instance_layer_names();
    let enabled_extension_names = get_instance_extension_names();
    let create_info = vk::InstanceCreateInfo::builder()
        .enabled_layer_names(&enabled_layer_names)
        .enabled_extension_names(&enabled_extension_names)
        .application_info(&app_info)
        .build();
    let instance = entry.create_instance(&create_info, None)?;

    Ok((entry, instance))
}

#[cfg(all(target_os = "windows", debug_assertions))]
fn get_instance_extension_names() -> Vec<*const i8> {
    vec![
        khr::Surface::name().as_ptr(),
        khr::Win32Surface::name().as_ptr(),
    ]
}

#[cfg(debug_assertions)]
fn get_instance_layer_names() -> Vec<*const i8> {
    let extensions = vec!["VK_LAYER_KHRONOS_validation\0"];
    return unsafe { get_raw_strings(extensions) };
}

#[cfg(all(target_os = "windows", debug_assertions))]
fn get_device_extensions() -> Vec<*const i8> {
    let extensions = vec!["VK_KHR_swapchain\0"];
    return unsafe { get_raw_strings(extensions) };
}
