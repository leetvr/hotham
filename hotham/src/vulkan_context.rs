use crate::{hotham_error::HothamError, image::Image, Result, COLOR_FORMAT, DEPTH_FORMAT};
use anyhow::anyhow;
use ash::{
    version::{DeviceV1_0, EntryV1_0, InstanceV1_0},
    vk::{self, Handle},
    Device, Entry, Instance,
};
use openxr as xr;
use std::{fmt::Debug, intrinsics::transmute};
use std::{mem::size_of, ptr::copy};

#[derive(Clone)]
pub(crate) struct VulkanContext {
    pub entry: Entry,
    pub instance: Instance,
    pub physical_device: vk::PhysicalDevice,
    pub device: Device,
    pub command_pool: vk::CommandPool,
    pub queue_family_index: u32,
    pub graphics_queue: vk::Queue,
}

// NOTE: OpenXR created the instance / device etc. and is therefore the owner. We'll let it do the cleanup.
impl Drop for VulkanContext {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_command_pool(self.command_pool, None);
        }
    }
}

impl VulkanContext {
    pub fn create_from_xr_instance(
        xr_instance: &xr::Instance,
        system: xr::SystemId,
    ) -> Result<Self> {
        print!("[HOTHAM_VULKAN] Creating VulkanContext..");
        let vk_target_version_xr = xr::Version::new(1, 1, 0);

        let requirements = xr_instance.graphics_requirements::<xr::Vulkan>(system)?;
        if vk_target_version_xr < requirements.min_api_version_supported
            || vk_target_version_xr.major() > requirements.max_api_version_supported.major()
        {
            return Err(HothamError::UnsupportedVersionError.into());
        }

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

        let graphics_queue_create_info = vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(queue_family_index)
            .queue_priorities(&[1.0])
            .build();
        let queue_create_infos = [graphics_queue_create_info];
        let multiview = &mut vk::PhysicalDeviceVulkan11Features {
            multiview: vk::TRUE,
            ..Default::default()
        };
        let separate_depth_stencil_layouts = &mut vk::PhysicalDeviceVulkan12Features {
            separate_depth_stencil_layouts: vk::TRUE,
            ..Default::default()
        };

        let device_create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_create_infos)
            .push_next(separate_depth_stencil_layouts)
            .push_next(multiview);

        let device_handle = unsafe {
            xr_instance.create_vulkan_device(
                system,
                get_instance_proc_addr,
                physical_device.as_raw() as *const _,
                &device_create_info as *const _ as *const _,
            )
        }?
        .map_err(vk::Result::from_raw)?;

        let device =
            unsafe { Device::load(instance.fp_v1_0(), vk::Device::from_raw(device_handle as _)) };

        let graphics_queue = unsafe { device.get_device_queue(queue_family_index, 0) };

        let command_pool = unsafe {
            device.create_command_pool(
                &vk::CommandPoolCreateInfo::builder()
                    .queue_family_index(queue_family_index)
                    .flags(
                        vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER
                            | vk::CommandPoolCreateFlags::TRANSIENT,
                    ),
                None,
            )
        }?;

        println!(" ..done!");

        Ok(Self {
            entry,
            instance,
            device,
            physical_device,
            command_pool,
            queue_family_index,
            graphics_queue,
        })
    }

    pub fn create_image_view(
        &self,
        image: &vk::Image,
        format: vk::Format,
    ) -> Result<vk::ImageView> {
        let aspect_mask = get_aspect_mask(format)?;
        let create_info = vk::ImageViewCreateInfo::builder()
            .view_type(vk::ImageViewType::TYPE_2D_ARRAY)
            .format(format)
            .components(vk::ComponentMapping {
                r: vk::ComponentSwizzle::R,
                g: vk::ComponentSwizzle::G,
                b: vk::ComponentSwizzle::B,
                a: vk::ComponentSwizzle::A,
            })
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 2,
            })
            .image(*image);
        unsafe { self.device.create_image_view(&create_info, None) }.map_err(Into::into)
    }

    pub fn create_image(&self, format: vk::Format, extent: &vk::Extent2D) -> Result<Image> {
        let usage = get_usage(format)?;
        let create_info = vk::ImageCreateInfo::builder()
            .format(format)
            .image_type(vk::ImageType::TYPE_2D)
            .extent(vk::Extent3D {
                height: extent.height,
                width: extent.width,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(usage)
            .samples(vk::SampleCountFlags::TYPE_1)
            .sharing_mode(vk::SharingMode::EXCLUSIVE); // TODO: multiview;
        let image = unsafe { self.device.create_image(&create_info, None) }?;

        let device_memory = self.allocate_image_memory(image)?;

        unsafe { self.device.bind_image_memory(image, device_memory, 0) }?;

        let image_view = self.create_image_view(&image, format)?;

        Ok(Image::new(image, image_view, device_memory, *extent))
    }

    pub fn create_buffer_with_data<T: Sized>(
        &self,
        data: &Vec<T>,
        usage: vk::BufferUsageFlags,
    ) -> Result<(vk::Buffer, vk::DeviceMemory)> {
        let device = &self.device;
        let size = (size_of::<T>() * data.len()) as _;
        let buffer_create_info = vk::BufferCreateInfo::builder()
            .size(size)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .usage(usage);

        let buffer = unsafe { device.create_buffer(&buffer_create_info, None) }?;
        let device_memory = self.allocate_buffer_memory(buffer)?;
        unsafe { device.bind_buffer_memory(buffer, device_memory, 0) }?;
        let dst =
            unsafe { device.map_memory(device_memory, 0, size, vk::MemoryMapFlags::empty()) }?;
        unsafe { copy(data.as_ptr(), dst as *mut _, data.len()) };

        Ok((buffer, device_memory))
    }

    pub fn find_memory_type(
        &self,
        type_filter: u32,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<u32> {
        let device_memory_properties = unsafe {
            self.instance
                .get_physical_device_memory_properties(self.physical_device)
        };
        for i in 0..device_memory_properties.memory_type_count {
            let has_type = type_filter & (1 << i) != 0;
            let has_properties = device_memory_properties.memory_types[i as usize]
                .property_flags
                .contains(properties);
            if has_type && has_properties {
                return Ok(i);
            }
        }

        Err(anyhow!(
            "Could not find a valid memory type for {:?}",
            properties
        ))
    }

    fn allocate_buffer_memory(&self, buffer: vk::Buffer) -> Result<vk::DeviceMemory> {
        let memory_requirements = unsafe { self.device.get_buffer_memory_requirements(buffer) };
        let properties =
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
        self.allocate_memory(memory_requirements, properties)
    }

    fn allocate_image_memory(&self, image: vk::Image) -> Result<vk::DeviceMemory> {
        let memory_requirements = unsafe { self.device.get_image_memory_requirements(image) };
        let properties = vk::MemoryPropertyFlags::DEVICE_LOCAL;
        self.allocate_memory(memory_requirements, properties)
    }

    fn allocate_memory(
        &self,
        memory_requirements: vk::MemoryRequirements,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<vk::DeviceMemory> {
        // Get memory requirements
        let memory_type_index =
            self.find_memory_type(memory_requirements.memory_type_bits, properties)?;

        let allocate_info = vk::MemoryAllocateInfo::builder()
            .memory_type_index(memory_type_index)
            .allocation_size(memory_requirements.size);

        unsafe { self.device.allocate_memory(&allocate_info, None) }.map_err(Into::into)
    }
}

fn get_usage(format: vk::Format) -> Result<vk::ImageUsageFlags> {
    if format == COLOR_FORMAT {
        return Ok(vk::ImageUsageFlags::COLOR_ATTACHMENT);
    }

    if format == DEPTH_FORMAT {
        return Ok(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT);
    }

    return Err(HothamError::InvalidFormatError.into());
}

fn get_aspect_mask(format: vk::Format) -> Result<vk::ImageAspectFlags> {
    if format == COLOR_FORMAT {
        return Ok(vk::ImageAspectFlags::COLOR);
    }

    if format == DEPTH_FORMAT {
        return Ok(vk::ImageAspectFlags::DEPTH);
    }

    return Err(HothamError::InvalidFormatError.into());
}

impl Debug for VulkanContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VulkanContext")
            .field("entry", &"Vulkan Entry".to_string())
            .field("instance", &"Vulkan Entry".to_string())
            .finish()
    }
}
