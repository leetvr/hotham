use crate::{
    hotham_error::HothamError, image::Image, util::parse_raw_strings, COLOR_FORMAT, DEPTH_FORMAT,
    SWAPCHAIN_LENGTH, TEXTURE_FORMAT,
};
use anyhow::{anyhow, Result};
use ash::{
    version::{DeviceV1_0, EntryV1_0, InstanceV1_0},
    vk::{self, Handle},
    Device, Entry, Instance as AshInstance,
};
use openxr as xr;
use std::{ffi::CString, fmt::Debug, intrinsics::transmute, os::raw::c_char};
use std::{mem::size_of, ptr::copy};
use xr::SystemId;

#[derive(Clone)]
pub(crate) struct VulkanContext {
    pub entry: Entry,
    pub instance: AshInstance,
    pub physical_device: vk::PhysicalDevice,
    pub device: Device,
    pub command_pool: vk::CommandPool,
    pub queue_family_index: u32,
    pub graphics_queue: vk::Queue,
    pub descriptor_pool: vk::DescriptorPool,
}

// NOTE: OpenXR created the instance / device etc. and is therefore the owner. We'll let it do the cleanup.
impl Drop for VulkanContext {
    fn drop(&mut self) {
        unsafe {
            self.device
                .destroy_descriptor_pool(self.descriptor_pool, None);
            self.device.destroy_command_pool(self.command_pool, None);
        }
    }
}

impl VulkanContext {
    #[cfg(not(target_os = "android"))]
    pub fn create_from_xr_instance(
        xr_instance: &xr::Instance,
        system: xr::SystemId,
    ) -> Result<Self> {
        println!("[HOTHAM_VULKAN] Creating VulkanContext..");
        let vk_target_version_xr = xr::Version::new(1, 2, 0);

        let requirements = xr_instance.graphics_requirements::<xr::VulkanLegacy>(system)?;
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
            AshInstance::load(
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

        let descriptor_pool = unsafe {
            device.create_descriptor_pool(
                &vk::DescriptorPoolCreateInfo::builder()
                    .pool_sizes(&[vk::DescriptorPoolSize {
                        ty: vk::DescriptorType::UNIFORM_BUFFER,
                        descriptor_count: SWAPCHAIN_LENGTH as _,
                        ..Default::default()
                    }])
                    .max_sets(SWAPCHAIN_LENGTH as _),
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
            descriptor_pool,
        })
    }

    pub fn create_from_xr_instance_legacy(
        xr_instance: &xr::Instance,
        system: xr::SystemId,
    ) -> Result<Self> {
        let vk_target_version_xr = xr::Version::new(1, 2, 0);

        let requirements = xr_instance.graphics_requirements::<xr::VulkanLegacy>(system)?;
        if vk_target_version_xr < requirements.min_api_version_supported
            || vk_target_version_xr.major() > requirements.max_api_version_supported.major()
        {
            return Err(HothamError::UnsupportedVersionError.into());
        }

        let (vulkan_instance, vulkan_entry) = vulkan_init_legacy(xr_instance, system)?;
        let physical_device = unsafe {
            xr_instance
                .get_vulkan_legacy_physical_device(system, transmute(vulkan_instance.handle()))
        }?;
        let physical_device = vk::PhysicalDevice::from_raw(physical_device as _);
        let (device, graphics_queue, queue_family_index) =
            create_vulkan_device_legacy(xr_instance, system, &vulkan_instance, physical_device)?;

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

        let descriptor_pool = unsafe {
            device.create_descriptor_pool(
                &vk::DescriptorPoolCreateInfo::builder()
                    .pool_sizes(&[vk::DescriptorPoolSize {
                        ty: vk::DescriptorType::UNIFORM_BUFFER,
                        descriptor_count: SWAPCHAIN_LENGTH as _,
                        ..Default::default()
                    }])
                    .max_sets(SWAPCHAIN_LENGTH as _),
                None,
            )
        }?;

        Ok(Self {
            entry: vulkan_entry,
            instance: vulkan_instance,
            physical_device,
            device,
            graphics_queue,
            queue_family_index,
            command_pool,
            descriptor_pool,
        })
    }

    pub fn create_image_view(
        &self,
        image: &vk::Image,
        format: vk::Format,
    ) -> Result<vk::ImageView> {
        let layer_count = if format == vk::Format::R8G8B8A8_SRGB {
            1
        } else {
            2
        };
        let aspect_mask = get_aspect_mask(format)?;
        let create_info = vk::ImageViewCreateInfo::builder()
            .view_type(vk::ImageViewType::TYPE_2D_ARRAY)
            .format(format)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count,
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
            .array_layers(2)
            .tiling(vk::ImageTiling::OPTIMAL)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(usage)
            .samples(vk::SampleCountFlags::TYPE_1)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        let image = unsafe { self.device.create_image(&create_info, None) }?;

        let properties = vk::MemoryPropertyFlags::DEVICE_LOCAL;

        let device_memory = self.allocate_image_memory(image, properties)?;

        unsafe { self.device.bind_image_memory(image, device_memory, 0) }?;

        let image_view = self.create_image_view(&image, format)?;

        Ok(Image::new(image, image_view, device_memory, *extent))
    }

    pub fn create_buffer_with_data<T: Sized>(
        &self,
        data: *const T,
        usage: vk::BufferUsageFlags,
        item_count: usize,
    ) -> Result<(vk::Buffer, vk::DeviceMemory)> {
        let buffer_size = (size_of::<T>() * item_count) as _;
        let device = &self.device;
        let buffer_create_info = vk::BufferCreateInfo::builder()
            .size(buffer_size)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .usage(usage);

        let buffer = unsafe { device.create_buffer(&buffer_create_info, None) }?;
        let device_memory = self.allocate_buffer_memory(buffer)?;
        unsafe { device.bind_buffer_memory(buffer, device_memory, 0) }?;
        self.update_buffer(data, item_count, device_memory)?;

        Ok((buffer, device_memory))
    }

    pub fn update_buffer<T: Sized>(
        &self,
        data: *const T,
        item_count: usize,
        device_memory: vk::DeviceMemory,
    ) -> Result<()> {
        let buffer_size = (size_of::<T>() * item_count) as _;
        let dst = unsafe {
            self.device
                .map_memory(device_memory, 0, buffer_size, vk::MemoryMapFlags::empty())
        }?;

        unsafe {
            copy(data, dst as *mut _, item_count);
            self.device.unmap_memory(device_memory);
        };

        Ok(())
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

    fn allocate_image_memory(
        &self,
        image: vk::Image,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<vk::DeviceMemory> {
        let memory_requirements = unsafe { self.device.get_image_memory_requirements(image) };
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

    pub fn create_texture_image(
        &self,
        image_buf: &Vec<u8>,
        width: u32,
        height: u32,
    ) -> Result<(Image, vk::Sampler)> {
        let size = image_buf.len() * 8;
        let image_extent = vk::Extent2D { width, height };

        let format = vk::Format::R8G8B8A8_SRGB;

        let image = self.create_image(format, &image_extent)?;

        let old_layout = vk::ImageLayout::UNDEFINED;
        let final_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;

        self.update_buffer(&image_buf, size, image.device_memory)?;
        self.transition_image_layout(image.handle, format, old_layout, final_layout);

        let sampler = self.create_texture_sampler()?;

        Ok((image, sampler))
    }

    pub fn transition_image_layout(
        &self,
        image: vk::Image,
        _format: vk::Format,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
    ) {
        let command_buffer = self.begin_single_time_commands();
        let subresource_range = vk::ImageSubresourceRange::builder()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .base_mip_level(0)
            .level_count(1)
            .base_array_layer(0)
            .layer_count(1)
            .build();

        let (src_access_mask, dst_access_mask, src_stage, dst_stage) =
            get_stage(old_layout, new_layout);

        let barrier = vk::ImageMemoryBarrier::builder()
            .old_layout(old_layout)
            .new_layout(new_layout)
            .src_access_mask(src_access_mask)
            .dst_access_mask(dst_access_mask)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .subresource_range(subresource_range)
            .image(image)
            .build();

        let dependency_flags = vk::DependencyFlags::empty();
        let image_memory_barriers = &[barrier];

        unsafe {
            self.device.cmd_pipeline_barrier(
                command_buffer,
                src_stage,
                dst_stage,
                dependency_flags,
                &[],
                &[],
                image_memory_barriers,
            )
        };
        self.end_single_time_commands(command_buffer);
    }
    pub fn begin_single_time_commands(&self) -> vk::CommandBuffer {
        let alloc_info = vk::CommandBufferAllocateInfo::builder()
            .command_buffer_count(1)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_pool(self.command_pool);

        let command_buffer = unsafe {
            self.device
                .allocate_command_buffers(&alloc_info)
                .map(|mut b| b.pop().unwrap())
                .expect("Unable to allocate command buffer")
        };

        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            self.device
                .begin_command_buffer(command_buffer, &begin_info)
                .expect("Unable to begin command buffer")
        }

        command_buffer
    }

    pub fn end_single_time_commands(&self, command_buffer: vk::CommandBuffer) {
        unsafe {
            self.device
                .end_command_buffer(command_buffer)
                .expect("Unable to end command buffer");
        }

        let command_buffers = &[command_buffer];

        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(command_buffers)
            .build();

        let submit_info = &[submit_info];

        unsafe {
            self.device
                .queue_submit(self.graphics_queue, submit_info, vk::Fence::null())
                .expect("Unable to submit to queue");
            self.device
                .queue_wait_idle(self.graphics_queue)
                .expect("Unable to wait idle");
            self.device
                .free_command_buffers(self.command_pool, command_buffers)
        }
    }

    fn create_texture_sampler(&self) -> Result<vk::Sampler> {
        let create_info = vk::SamplerCreateInfo::builder()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::REPEAT)
            .address_mode_v(vk::SamplerAddressMode::REPEAT)
            .address_mode_w(vk::SamplerAddressMode::REPEAT)
            .anisotropy_enable(true)
            .max_anisotropy(16.0)
            .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
            .unnormalized_coordinates(false)
            .compare_enable(false)
            .compare_op(vk::CompareOp::ALWAYS)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .mip_lod_bias(0.0)
            .min_lod(0.0)
            .max_lod(0.0)
            .build();

        unsafe {
            self.device
                .create_sampler(&create_info, None)
                .map_err(Into::into)
        }
    }
}

fn get_usage(format: vk::Format) -> Result<vk::ImageUsageFlags> {
    if format == COLOR_FORMAT {
        return Ok(vk::ImageUsageFlags::COLOR_ATTACHMENT);
    }

    if format == DEPTH_FORMAT {
        return Ok(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT);
    }

    if format == TEXTURE_FORMAT {
        return Ok(vk::ImageUsageFlags::SAMPLED);
    }

    return Err(HothamError::InvalidFormatError.into());
}

fn get_aspect_mask(format: vk::Format) -> Result<vk::ImageAspectFlags> {
    if format == COLOR_FORMAT || format == TEXTURE_FORMAT {
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

fn vulkan_init_legacy(instance: &xr::Instance, system: SystemId) -> Result<(AshInstance, Entry)> {
    println!("[HOTHAM_VULKAN] Initialising Vulkan..");
    let app_name = CString::new("Hotham Cubeworld")?;
    let entry = unsafe { Entry::new()? };
    let layer_names = get_layer_names(&entry)?;
    println!("[HOTHAM_VULKAN] Trying to use layers: {:?}", unsafe {
        parse_raw_strings(&layer_names)
    });

    let extension_names = unsafe { instance.get_vulkan_legacy_instance_extensions(system) }?;
    println!("[HOTHAM_VULKAN] Trying to use extensions: {:?}", {
        &extension_names
    });
    let extension_names = extension_names
        .iter()
        .map(|e| e.as_ptr())
        .collect::<Vec<_>>();

    let app_info = vk::ApplicationInfo::builder()
        .application_name(&app_name)
        .api_version(vk::make_version(1, 2, 0));
    let create_info = vk::InstanceCreateInfo::builder()
        .application_info(&app_info)
        .enabled_extension_names(&extension_names)
        .enabled_layer_names(&layer_names);

    let instance = unsafe { entry.create_instance(&create_info, None) }?;

    println!("[HOTHAM_VULKAN] ..done");

    Ok((instance, entry))
}

pub fn create_vulkan_device_legacy(
    xr_instance: &xr::Instance,
    system: SystemId,
    vulkan_instance: &AshInstance,
    physical_device: vk::PhysicalDevice,
) -> Result<(Device, vk::Queue, u32)> {
    println!("[HOTHAM_VULKAN] Creating logical device.. ");
    let mut extension_names = unsafe { xr_instance.get_vulkan_legacy_device_extensions(system) }?;
    unsafe {
        extension_names.push(CString::from_vec_unchecked(b"VK_KHR_multiview".to_vec()));
    }

    println!(
        "[HOTHAM_VULKAN] Using device extensions: {:?}",
        extension_names
    );
    let extension_names = extension_names
        .iter()
        .map(|e| e.as_ptr())
        .collect::<Vec<_>>();
    let queue_priorities = [1.0];
    let graphics_family_index = unsafe {
        vulkan_instance
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
        .queue_priorities(&queue_priorities)
        .queue_family_index(graphics_family_index)
        .build();

    let queue_create_infos = [graphics_queue_create_info];

    let physical_device_features = vk::PhysicalDeviceFeatures::builder();
    // TODO: Quest 2?
    // physical_device_features.shader_storage_image_multisample(true);

    let multiview = &mut vk::PhysicalDeviceVulkan11Features {
        multiview: vk::TRUE,
        ..Default::default()
    };

    let device_create_info = vk::DeviceCreateInfo::builder()
        .queue_create_infos(&queue_create_infos)
        .enabled_extension_names(&extension_names)
        .enabled_features(&physical_device_features)
        .push_next(multiview);

    let device =
        unsafe { vulkan_instance.create_device(physical_device, &device_create_info, None) }?;

    let graphics_queue = unsafe { device.get_device_queue(graphics_family_index, 0) };

    println!("[HOTHAM_VULKAN] ..done");

    Ok((device, graphics_queue, graphics_family_index))
}

fn get_layer_names(entry: &Entry) -> Result<Vec<*const c_char>> {
    Ok(entry
        .enumerate_instance_layer_properties()?
        .iter()
        .map(|l| l.layer_name.as_ptr())
        .collect::<Vec<_>>())
}

fn get_stage(
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
) -> (
    vk::AccessFlags,
    vk::AccessFlags,
    vk::PipelineStageFlags,
    vk::PipelineStageFlags,
) {
    if old_layout == vk::ImageLayout::UNDEFINED
        && new_layout == vk::ImageLayout::TRANSFER_DST_OPTIMAL
    {
        return (
            vk::AccessFlags::empty(),
            vk::AccessFlags::TRANSFER_WRITE,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::TRANSFER,
        );
    } else if old_layout == vk::ImageLayout::TRANSFER_DST_OPTIMAL
        && new_layout == vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
    {
        return (
            vk::AccessFlags::TRANSFER_WRITE,
            vk::AccessFlags::SHADER_READ,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
        );
    }

    panic!("Invalid layout transition!");
}
