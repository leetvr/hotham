use crate::{
    buffer::Buffer,
    hotham_error::HothamError,
    image::Image,
    scene_data::{SceneData, SceneParams},
    texture::Texture,
    DEPTH_ATTACHMENT_USAGE_FLAGS, DEPTH_FORMAT,
};
use anyhow::{anyhow, Result};
use ash::{
    extensions::ext::DebugUtils,
    prelude::VkResult,
    util::Align,
    vk::{self, Handle, ObjectType},
    Device, Entry, Instance as AshInstance,
};
use openxr as xr;
use std::{cmp::max, ffi::CString, fmt::Debug, ptr::copy, slice::from_ref as slice_from_ref};

type XrVulkan = xr::Vulkan;

#[cfg(debug_assertions)]
use ash::vk::DebugUtilsObjectNameInfoEXT;

#[derive(Clone)]
pub struct VulkanContext {
    pub entry: Entry,
    pub instance: AshInstance,
    pub physical_device: vk::PhysicalDevice,
    pub device: Device,
    pub command_pool: vk::CommandPool,
    pub queue_family_index: u32,
    pub graphics_queue: vk::Queue,
    pub descriptor_pool: vk::DescriptorPool,
    pub debug_utils: DebugUtils,
    pub physical_device_properties: vk::PhysicalDeviceProperties,
}

impl VulkanContext {
    #[cfg(not(target_os = "android"))]
    #[allow(unused)]
    pub fn create_from_xr_instance(
        xr_instance: &xr::Instance,
        system: xr::SystemId,
        application_name: &str,
        application_version: u32,
    ) -> Result<Self> {
        println!("[HOTHAM_VULKAN] Creating VulkanContext..");
        let vk_target_version_xr = xr::Version::new(1, 2, 0);

        let requirements = xr_instance.graphics_requirements::<XrVulkan>(system)?;
        if vk_target_version_xr < requirements.min_api_version_supported
            || vk_target_version_xr.major() > requirements.max_api_version_supported.major()
        {
            return Err(HothamError::UnsupportedVersionError.into());
        }

        let entry = unsafe { Entry::new() }?;
        let get_instance_proc_addr =
            unsafe { std::mem::transmute(entry.static_fn().get_instance_proc_addr) };

        let app_name = CString::new(application_name)?;
        let engine_name = CString::new("Hotham")?;
        let app_info = vk::ApplicationInfo::builder()
            .api_version(vk::make_api_version(0, 1, 2, 0))
            .application_name(&app_name)
            .application_version(application_version)
            .engine_name(&engine_name)
            .engine_version(1)
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

        let command_pool = create_command_pool(&device, queue_family_index)?;

        let descriptor_pool = create_descriptor_pool(&device)?;
        let debug_utils = DebugUtils::new(&entry, &instance);
        let physical_device_properties =
            unsafe { instance.get_physical_device_properties(physical_device) };

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
            debug_utils,
            physical_device_properties,
        })
    }

    pub fn create_from_xr_instance_legacy(
        xr_instance: &xr::Instance,
        system: xr::SystemId,
        application_name: &str,
        application_version: u32,
    ) -> Result<Self> {
        let vk_target_version_xr = xr::Version::new(1, 2, 0);

        let requirements = xr_instance.graphics_requirements::<XrVulkan>(system)?;
        if vk_target_version_xr < requirements.min_api_version_supported
            || vk_target_version_xr.major() > requirements.max_api_version_supported.major()
        {
            return Err(HothamError::UnsupportedVersionError.into());
        }

        let (vulkan_instance, vulkan_entry) =
            vulkan_init_legacy(xr_instance, system, application_name, application_version)?;
        let physical_device = vk::PhysicalDevice::from_raw(
            xr_instance
                .vulkan_graphics_device(system, vulkan_instance.handle().as_raw() as _)
                .unwrap() as _,
        );
        let (device, graphics_queue, queue_family_index) =
            create_vulkan_device_legacy(xr_instance, system, &vulkan_instance, physical_device)?;

        let command_pool = create_command_pool(&device, queue_family_index)?;

        let descriptor_pool = create_descriptor_pool(&device)?;
        let debug_utils = DebugUtils::new(&vulkan_entry, &vulkan_instance);
        let physical_device_properties =
            unsafe { vulkan_instance.get_physical_device_properties(physical_device) };

        Ok(Self {
            entry: vulkan_entry,
            instance: vulkan_instance,
            physical_device,
            device,
            graphics_queue,
            queue_family_index,
            command_pool,
            descriptor_pool,
            debug_utils,
            physical_device_properties,
        })
    }

    pub fn testing() -> Result<Self> {
        let (instance, entry) = vulkan_init_test()?;
        let physical_device = get_test_physical_device(&instance);
        let mut extension_names = Vec::new();
        add_device_extension_names(&mut extension_names);

        let (device, graphics_queue, queue_family_index) =
            create_vulkan_device(&extension_names, &instance, physical_device)?;

        let command_pool = create_command_pool(&device, queue_family_index)?;
        let descriptor_pool = create_descriptor_pool(&device)?;
        let debug_utils = DebugUtils::new(&entry, &instance);
        let physical_device_properties =
            unsafe { instance.get_physical_device_properties(physical_device) };

        Ok(Self {
            entry,
            instance,
            physical_device,
            device,
            graphics_queue,
            queue_family_index,
            command_pool,
            descriptor_pool,
            debug_utils,
            physical_device_properties,
        })
    }

    pub fn create_image_view(
        &self,
        image: &vk::Image,
        format: vk::Format,
        view_type: vk::ImageViewType,
        layer_count: u32,
        mip_levels: u32,
    ) -> Result<vk::ImageView> {
        let aspect_mask = get_aspect_mask(format);
        let create_info = vk::ImageViewCreateInfo::builder()
            .view_type(view_type)
            .format(format)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask,
                base_mip_level: 0,
                level_count: mip_levels,
                base_array_layer: 0,
                layer_count,
            })
            .components(vk::ComponentMapping {
                r: vk::ComponentSwizzle::R,
                g: vk::ComponentSwizzle::G,
                b: vk::ComponentSwizzle::B,
                a: vk::ComponentSwizzle::A,
            })
            .image(*image);
        unsafe { self.device.create_image_view(&create_info, None) }.map_err(Into::into)
    }

    pub fn create_image(
        &self,
        format: vk::Format,
        extent: &vk::Extent2D,
        usage: vk::ImageUsageFlags,
        array_layers: u32,
        mip_levels: u32,
    ) -> Result<Image> {
        let tiling = vk::ImageTiling::OPTIMAL;
        let (flags, image_view_type) = if array_layers == 1 {
            (vk::ImageCreateFlags::empty(), vk::ImageViewType::TYPE_2D)
        } else if array_layers == 6 {
            (
                vk::ImageCreateFlags::CUBE_COMPATIBLE,
                vk::ImageViewType::CUBE,
            )
        } else {
            (
                vk::ImageCreateFlags::empty(),
                vk::ImageViewType::TYPE_2D_ARRAY,
            )
        };
        let samples = if usage.contains(vk::ImageUsageFlags::TRANSIENT_ATTACHMENT)
            || usage.contains(DEPTH_ATTACHMENT_USAGE_FLAGS)
        {
            vk::SampleCountFlags::TYPE_4
        } else {
            vk::SampleCountFlags::TYPE_1
        };

        let create_info = vk::ImageCreateInfo::builder()
            .format(format)
            .image_type(vk::ImageType::TYPE_2D)
            .extent(vk::Extent3D {
                width: extent.width,
                height: extent.height,
                depth: 1,
            })
            .flags(flags)
            .mip_levels(mip_levels)
            .array_layers(array_layers)
            .tiling(tiling)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(usage)
            .samples(samples)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        let image = unsafe { self.device.create_image(&create_info, None) }?;

        let (_, device_memory) = self.allocate_image_memory(image)?;

        unsafe { self.device.bind_image_memory(image, device_memory, 0) }?;

        let image_view =
            self.create_image_view(&image, format, image_view_type, array_layers, mip_levels)?;

        Ok(Image::new(
            image,
            image_view,
            device_memory,
            *extent,
            usage,
            format,
            image_view_type,
            array_layers,
        ))
    }

    /// Create a Vulkan buffer filled with the contents of `data`.
    pub fn create_buffer_with_data<T: Sized + Copy>(
        &self,
        data: &[T],
        usage: vk::BufferUsageFlags,
        buffer_size: vk::DeviceSize,
    ) -> Result<(vk::Buffer, vk::DeviceMemory, vk::DeviceSize)> {
        let device = &self.device;
        let buffer_create_info = vk::BufferCreateInfo::builder()
            .size(buffer_size)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .usage(usage);

        let buffer = unsafe { device.create_buffer(&buffer_create_info, None) }?;
        let (device_memory_size, device_memory) = self.allocate_buffer_memory(buffer)?;

        println!(
            "[HOTHAM_VULKAN] Allocated {} bits of buffer memory: {:?}",
            device_memory_size, device_memory
        );
        unsafe { device.bind_buffer_memory(buffer, device_memory, 0) }?;
        self.update_buffer(data, device_memory, buffer_size, usage)?;

        Ok((buffer, device_memory, device_memory_size))
    }

    pub fn update_buffer<T: Sized + Copy>(
        &self,
        data: &[T],
        device_memory: vk::DeviceMemory,
        device_memory_size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
    ) -> Result<()> {
        unsafe {
            let dst = self.device.map_memory(
                device_memory,
                0,
                vk::WHOLE_SIZE,
                vk::MemoryMapFlags::empty(),
            )?;

            if usage == vk::BufferUsageFlags::UNIFORM_BUFFER {
                let (alignment, aligned_size) = self.get_alignment_info::<T>(device_memory_size);
                let mut align = Align::new(dst, alignment, aligned_size);
                align.copy_from_slice(data);
            } else {
                copy(data.as_ptr(), dst as *mut _, data.len())
            }
            self.device.unmap_memory(device_memory);
        };

        Ok(())
    }

    pub fn get_alignment_info<T: Sized>(
        &self,
        original_size: vk::DeviceSize,
    ) -> (vk::DeviceSize, vk::DeviceSize) {
        let min_alignment = self
            .physical_device_properties
            .limits
            .min_uniform_buffer_offset_alignment;
        let alignment = max(std::mem::align_of::<T>() as vk::DeviceSize, min_alignment);

        let aligned_size = (original_size + min_alignment - 1) & !(min_alignment - 1);

        (alignment, aligned_size)
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

    fn allocate_buffer_memory(
        &self,
        buffer: vk::Buffer,
    ) -> Result<(vk::DeviceSize, vk::DeviceMemory)> {
        let memory_requirements = unsafe { self.device.get_buffer_memory_requirements(buffer) };
        // PERF: This is slow.
        let properties =
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
        self.allocate_memory(memory_requirements, properties)
    }

    fn allocate_image_memory(
        &self,
        image: vk::Image,
    ) -> Result<(vk::DeviceSize, vk::DeviceMemory)> {
        let properties = vk::MemoryPropertyFlags::DEVICE_LOCAL;
        let memory_requirements = unsafe { self.device.get_image_memory_requirements(image) };
        self.allocate_memory(memory_requirements, properties)
    }

    fn allocate_memory(
        &self,
        memory_requirements: vk::MemoryRequirements,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<(vk::DeviceSize, vk::DeviceMemory)> {
        // Get memory requirements
        let memory_type_index =
            self.find_memory_type(memory_requirements.memory_type_bits, properties)?;

        let allocate_info = vk::MemoryAllocateInfo::builder()
            .memory_type_index(memory_type_index)
            .allocation_size(memory_requirements.size);

        let device_memory = unsafe { self.device.allocate_memory(&allocate_info, None) }?;

        Ok((memory_requirements.size, device_memory))
    }

    pub fn create_texture_image(
        &self,
        name: &str,
        image_buf: &[u8], // Clippy &Vec<u8>, ptr_arg for texture.rs
        mip_count: u32,
        offsets: Vec<vk::DeviceSize>,
        texture_image: Image,
    ) -> Result<(Image, vk::Sampler)> {
        // Get the image's properties
        let layer_count = texture_image.layer_count;
        let format = texture_image.format;

        self.set_debug_name(vk::ObjectType::IMAGE, texture_image.handle.as_raw(), name)?;

        // Create a staging buffer.
        println!("[HOTHAM_VULKAN] Creating staging buffer..");
        let usage = vk::BufferUsageFlags::TRANSFER_SRC;
        let size = 8 * image_buf.len();
        let (staging_buffer, staging_memory, _) =
            self.create_buffer_with_data(image_buf, usage, size as _)?;
        println!("[HOTHAM_VULKAN] ..done!");

        // Copy the buffer into the image
        let initial_layout = vk::ImageLayout::UNDEFINED;
        let transfer_layout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
        self.transition_image_layout(
            texture_image.handle,
            initial_layout,
            transfer_layout,
            layer_count,
            mip_count,
        );

        println!("[HOTHAM_VULKAN] Copying buffer to image..");
        self.copy_buffer_to_image(
            staging_buffer,
            &texture_image,
            layer_count,
            mip_count,
            offsets,
        );

        // Now transition the image
        println!("[HOTHAM_VULKAN] ..done! Transitioning image layout..");
        let final_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
        self.transition_image_layout(
            texture_image.handle,
            transfer_layout,
            final_layout,
            layer_count,
            mip_count,
        );
        println!("[HOTHAM_VULKAN] ..done! Freeing staging buffer..");
        let sampler_address_mode = if format == vk::Format::R16G16_SFLOAT || layer_count == 6 {
            vk::SamplerAddressMode::CLAMP_TO_EDGE
        } else {
            vk::SamplerAddressMode::REPEAT
        };

        let sampler = self.create_texture_sampler(sampler_address_mode, mip_count)?;
        self.set_debug_name(vk::ObjectType::SAMPLER, sampler.as_raw(), name)?;

        // Free the staging buffer
        unsafe {
            self.device.destroy_buffer(staging_buffer, None);
            self.device.free_memory(staging_memory, None);
        }

        println!(
            "[HOTHAM_VULKAN] ..done! Texture {} created successfully.",
            name
        );

        Ok((texture_image, sampler))
    }

    pub fn transition_image_layout(
        &self,
        image: vk::Image,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
        layer_count: u32,
        mip_count: u32,
    ) {
        let command_buffer = self.begin_single_time_commands();
        let subresource_range = vk::ImageSubresourceRange::builder()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .base_mip_level(0)
            .level_count(mip_count)
            .base_array_layer(0)
            .layer_count(layer_count)
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

    pub fn create_texture_sampler(
        &self,
        address_mode: vk::SamplerAddressMode,
        mip_count: u32,
    ) -> Result<vk::Sampler> {
        let max_anisotropy = self
            .physical_device_properties
            .limits
            .max_sampler_anisotropy;
        let create_info = vk::SamplerCreateInfo::builder()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .address_mode_u(address_mode)
            .address_mode_v(address_mode)
            .address_mode_w(address_mode)
            .anisotropy_enable(true)
            .max_anisotropy(max_anisotropy)
            .border_color(vk::BorderColor::INT_OPAQUE_WHITE)
            .unnormalized_coordinates(false)
            .compare_enable(false)
            .compare_op(vk::CompareOp::NEVER)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .mip_lod_bias(0.0)
            .min_lod(0.0)
            .max_lod(mip_count as _)
            .build();

        unsafe {
            self.device
                .create_sampler(&create_info, None)
                .map_err(Into::into)
        }
    }

    pub fn copy_buffer_to_image(
        &self,
        src_buffer: vk::Buffer,
        dst_image: &Image,
        layer_count: u32,
        mip_count: u32,
        offsets: Vec<vk::DeviceSize>,
    ) {
        let command_buffer = self.begin_single_time_commands();

        let mut regions = Vec::new();
        for layer in 0..layer_count {
            for mip_level in 0..mip_count {
                let image_subresource = vk::ImageSubresourceLayers::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .mip_level(mip_level)
                    .base_array_layer(layer)
                    .layer_count(1);

                let image_extent = vk::Extent3D {
                    width: dst_image.extent.width >> mip_level,
                    height: dst_image.extent.height >> mip_level,
                    depth: 1,
                };
                let offset_index = (layer * mip_count) + mip_level;

                let region = vk::BufferImageCopy::builder()
                    .buffer_offset(offsets[offset_index as usize])
                    .buffer_row_length(0)
                    .buffer_image_height(0)
                    .image_subresource(*image_subresource)
                    .image_extent(image_extent)
                    .build();
                regions.push(region);
            }
        }

        let dst_image_layout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;

        unsafe {
            self.device.cmd_copy_buffer_to_image(
                command_buffer,
                src_buffer,
                dst_image.handle,
                dst_image_layout,
                &regions,
            )
        };

        self.end_single_time_commands(command_buffer);
    }

    pub fn copy_image_to_buffer(
        &self,
        src_image: &Image,
        src_image_layout: vk::ImageLayout,
        dst_buffer: vk::Buffer,
    ) {
        let command_buffer = self.begin_single_time_commands();
        let image_subresource = vk::ImageSubresourceLayers::builder()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .mip_level(0)
            .base_array_layer(0)
            .layer_count(1)
            .build();

        let image_extent = vk::Extent3D {
            width: src_image.extent.width,
            height: src_image.extent.height,
            depth: 1,
        };

        let region = vk::BufferImageCopy::builder()
            .buffer_offset(0)
            .buffer_row_length(0)
            .buffer_image_height(0)
            .image_subresource(image_subresource)
            .image_extent(image_extent);

        unsafe {
            self.device.cmd_copy_image_to_buffer(
                command_buffer,
                src_image.handle,
                src_image_layout,
                dst_buffer,
                &[*region],
            )
        };

        self.end_single_time_commands(command_buffer);
    }

    // TODO: These kind of smell - VulkanContext shouldn't know about application specific things.
    pub fn create_mesh_descriptor_sets(
        &self,
        set_layout: vk::DescriptorSetLayout,
        mesh_name: &str,
    ) -> VkResult<Vec<vk::DescriptorSet>> {
        println!("[HOTHAM_VULKAN] Allocating mesh descriptor sets..");
        let descriptor_sets = unsafe {
            self.device.allocate_descriptor_sets(
                &vk::DescriptorSetAllocateInfo::builder()
                    .set_layouts(&[set_layout])
                    .descriptor_pool(self.descriptor_pool),
            )
        }?;
        self.set_debug_name(
            vk::ObjectType::DESCRIPTOR_SET,
            descriptor_sets[0].as_raw(),
            &format!("Mesh {}", mesh_name),
        )?;
        println!("[HOTHAM_VULKAN] ..done! {:?}", descriptor_sets);

        Ok(descriptor_sets)
    }

    pub fn create_textures_descriptor_sets(
        &self,
        set_layout: vk::DescriptorSetLayout,
        material_name: &str,
        textures: &[&Texture; 5],
    ) -> VkResult<Vec<vk::DescriptorSet>> {
        println!("[HOTHAM_VULKAN] Allocating textures descriptor sets..");
        let descriptor_sets = unsafe {
            self.device.allocate_descriptor_sets(
                &vk::DescriptorSetAllocateInfo::builder()
                    .set_layouts(&[set_layout])
                    .descriptor_pool(self.descriptor_pool),
            )
        }?;

        self.set_debug_name(
            vk::ObjectType::DESCRIPTOR_SET,
            descriptor_sets[0].as_raw(),
            &format!("Material {}", material_name),
        )?;
        println!("[HOTHAM_VULKAN] ..done! {:?}", descriptor_sets);

        unsafe {
            /*
            let mut s = Vec::new();
            for (i, texture) in textures.iter().enumerate() {
                s.push(
                    *vk::WriteDescriptorSet::builder()
                        .dst_set(descriptor_sets[0])
                        .dst_binding(i as u32)
                        .dst_array_element(0)
                        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .image_info(&[*vk::DescriptorImageInfo::builder()
                            .image_view(texture.image.view)
                            .sampler(texture.sampler)
                            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)]),
                );
            }

            self.device
                .update_descriptor_sets(&[s[0], s[1], s[2], s[3], s[4]], &[]);
            */

            let base_color_texture = &textures[0];
            let metallic_roughness_texture = &textures[1];
            let normal_map = &textures[2];
            let ao_map = &textures[3];
            let emissive_map = &textures[4];

            let ds = &[
                *vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_sets[0])
                    .dst_binding(0)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(&[*vk::DescriptorImageInfo::builder()
                        .image_view(base_color_texture.image.view)
                        .sampler(base_color_texture.sampler)
                        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)]),
                *vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_sets[0])
                    .dst_binding(1)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(&[*vk::DescriptorImageInfo::builder()
                        .image_view(metallic_roughness_texture.image.view)
                        .sampler(metallic_roughness_texture.sampler)
                        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)]),
                *vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_sets[0])
                    .dst_binding(2)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(&[*vk::DescriptorImageInfo::builder()
                        .image_view(normal_map.image.view)
                        .sampler(normal_map.sampler)
                        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)]),
                *vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_sets[0])
                    .dst_binding(3)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(&[*vk::DescriptorImageInfo::builder()
                        .image_view(ao_map.image.view)
                        .sampler(ao_map.sampler)
                        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)]),
                *vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_sets[0])
                    .dst_binding(4)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(&[*vk::DescriptorImageInfo::builder()
                        .image_view(emissive_map.image.view)
                        .sampler(emissive_map.sampler)
                        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)]),
            ];
            self.device.update_descriptor_sets(ds, &[]);
        }
        Ok(descriptor_sets)
    }

    pub fn create_scene_data_descriptor_sets(
        &self,
        set_layout: vk::DescriptorSetLayout,
        scene_data: &Buffer<SceneData>,
        scene_params: &Buffer<SceneParams>,
        irradiance: &Texture,
        prefiltered_map: &Texture,
        brdflut: &Texture,
    ) -> VkResult<Vec<vk::DescriptorSet>> {
        println!("[HOTHAM_VULKAN] Allocating scene data sets..");
        let descriptor_sets = unsafe {
            self.device.allocate_descriptor_sets(
                &vk::DescriptorSetAllocateInfo::builder()
                    .set_layouts(&[set_layout])
                    .descriptor_pool(self.descriptor_pool),
            )
        }?;
        self.set_debug_name(
            vk::ObjectType::DESCRIPTOR_SET,
            descriptor_sets[0].as_raw(),
            "Scene Data",
        )?;

        self.update_buffer_descriptor_set(
            scene_data,
            descriptor_sets[0],
            0,
            vk::DescriptorType::UNIFORM_BUFFER,
        );
        self.update_buffer_descriptor_set(
            scene_params,
            descriptor_sets[0],
            1,
            vk::DescriptorType::UNIFORM_BUFFER,
        );
        unsafe {
            self.device.update_descriptor_sets(
                &[
                    *vk::WriteDescriptorSet::builder()
                        .dst_set(descriptor_sets[0])
                        .dst_binding(2)
                        .dst_array_element(0)
                        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .image_info(&[*vk::DescriptorImageInfo::builder()
                            .image_view(irradiance.image.view)
                            .sampler(irradiance.sampler)
                            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)]),
                    *vk::WriteDescriptorSet::builder()
                        .dst_set(descriptor_sets[0])
                        .dst_binding(3)
                        .dst_array_element(0)
                        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .image_info(&[*vk::DescriptorImageInfo::builder()
                            .image_view(prefiltered_map.image.view)
                            .sampler(prefiltered_map.sampler)
                            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)]),
                    *vk::WriteDescriptorSet::builder()
                        .dst_set(descriptor_sets[0])
                        .dst_binding(4)
                        .dst_array_element(0)
                        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .image_info(&[*vk::DescriptorImageInfo::builder()
                            .image_view(brdflut.image.view)
                            .sampler(brdflut.sampler)
                            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)]),
                ],
                &[],
            )
        }

        Ok(descriptor_sets)
    }

    pub fn update_buffer_descriptor_set<T>(
        &self,
        buffer: &Buffer<T>,
        descriptor_set: vk::DescriptorSet,
        binding: usize,
        descriptor_type: vk::DescriptorType,
    ) {
        unsafe {
            self.device.update_descriptor_sets(
                &[*vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set)
                    .dst_binding(binding as _)
                    .dst_array_element(0)
                    .descriptor_type(descriptor_type)
                    .buffer_info(&[*vk::DescriptorBufferInfo::builder()
                        .buffer(buffer.handle)
                        .offset(0)
                        .range(buffer.size)])],
                &[],
            )
        };
    }

    #[cfg(not(debug_assertions))]
    pub fn set_debug_name(
        &self,
        _object_type: ObjectType,
        _object_handle: u64,
        _object_name: &str,
    ) -> VkResult<()> {
        VkResult::Ok(())
    }

    #[cfg(debug_assertions)]
    pub fn set_debug_name(
        &self,
        object_type: ObjectType,
        object_handle: u64,
        object_name: &str,
    ) -> VkResult<()> {
        let object_name = CString::new(object_name).unwrap();
        unsafe {
            self.debug_utils.debug_utils_set_object_name(
                self.device.handle(),
                &*DebugUtilsObjectNameInfoEXT::builder()
                    .object_type(object_type)
                    .object_handle(object_handle)
                    .object_name(object_name.as_c_str()),
            )
        }
    }
}

#[allow(unused_variables)]
#[allow(clippy::ptr_arg)] // https://github.com/rust-lang/rust-clippy/issues/8388
fn add_device_extension_names(extension_names: &mut Vec<CString>) {
    extension_names.push(vk::KhrShaderDrawParametersFn::name().to_owned());

    // Add Multiview extension
    #[cfg(target_os = "android")]
    extension_names.push(CString::new("VK_KHR_multiview").unwrap());

    // If we're on macOS we've got to add portability
    #[cfg(target_os = "macos")]
    extension_names.push(CString::new("VK_KHR_portability_subset").unwrap());
}

fn create_command_pool(
    device: &Device,
    queue_family_index: u32,
) -> Result<vk::CommandPool, anyhow::Error> {
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
    Ok(command_pool)
}

// TODO HACK: Make these values real
fn create_descriptor_pool(device: &Device) -> Result<vk::DescriptorPool, anyhow::Error> {
    let descriptor_pool = unsafe {
        device.create_descriptor_pool(
            &vk::DescriptorPoolCreateInfo::builder()
                .pool_sizes(&[
                    vk::DescriptorPoolSize {
                        ty: vk::DescriptorType::UNIFORM_BUFFER,
                        descriptor_count: 100,
                    },
                    vk::DescriptorPoolSize {
                        ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                        descriptor_count: 1000,
                    },
                    vk::DescriptorPoolSize {
                        ty: vk::DescriptorType::STORAGE_BUFFER,
                        descriptor_count: 1000,
                    },
                ])
                .max_sets(1000),
            None,
        )
    }?;
    Ok(descriptor_pool)
}

fn get_aspect_mask(format: vk::Format) -> vk::ImageAspectFlags {
    if format == DEPTH_FORMAT {
        vk::ImageAspectFlags::DEPTH
    } else {
        vk::ImageAspectFlags::COLOR
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

fn vulkan_init_legacy(
    xr_instance: &xr::Instance,
    system: xr::SystemId,
    application_name: &str,
    application_version: u32,
) -> Result<(AshInstance, Entry)> {
    use crate::util::get_raw_strings;

    println!("[HOTHAM_VULKAN] Initializing Vulkan..");
    unsafe {
        let entry = Entry::new()?;

        #[cfg(debug_assertions)]
        let layers = vec!["VK_LAYER_KHRONOS_validation\0"];

        #[cfg(not(debug_assertions))]
        let layers = vec![];

        println!("[HOTHAM_VULKAN] Requesting layers: {:?}", layers);

        let layer_names = get_raw_strings(layers);

        #[allow(unused_mut)]
        let mut vk_instance_exts = xr_instance
            .vulkan_legacy_instance_extensions(system)
            .unwrap()
            .split(' ')
            .map(|x| CString::new(x).unwrap())
            .collect::<Vec<_>>();

        #[cfg(debug_assertions)]
        vk_instance_exts.push(vk::ExtDebugUtilsFn::name().to_owned());

        println!(
            "[HOTHAM_VULKAN] Required Vulkan instance extensions: {:?}",
            vk_instance_exts
        );
        let vk_instance_ext_pointers = vk_instance_exts
            .iter()
            .map(|x| x.as_ptr())
            .collect::<Vec<_>>();

        let app_name = CString::new(application_name)?;
        let engine_name = CString::new("Hotham")?;
        let app_info = vk::ApplicationInfo::builder()
            .application_name(&app_name)
            .application_version(application_version)
            .engine_name(&engine_name)
            .engine_version(1)
            .api_version(vk::make_api_version(0, 1, 2, 0));

        let validation_features_enables = [];
        let validation_features_disables = [];
        let mut validation_features = vk::ValidationFeaturesEXT::builder()
            .enabled_validation_features(&validation_features_enables)
            .disabled_validation_features(&validation_features_disables);

        let instance = entry
            .create_instance(
                &vk::InstanceCreateInfo::builder()
                    .application_info(&app_info)
                    .enabled_extension_names(&vk_instance_ext_pointers)
                    .enabled_layer_names(&layer_names)
                    .push_next(&mut validation_features),
                None,
            )
            .expect("Vulkan error creating Vulkan instance");

        Ok((instance, entry))
    }
}

fn vulkan_init_test() -> Result<(AshInstance, Entry)> {
    use crate::util::{get_raw_strings, parse_raw_strings};

    println!("[HOTHAM_VULKAN] Initializing Vulkan..");
    let app_name = CString::new("Hotham Testing")?;
    let entry = unsafe { Entry::new()? };
    let layers = vec!["VK_LAYER_KHRONOS_validation\0"];
    let layer_names = unsafe { get_raw_strings(layers) };
    println!("[HOTHAM_VULKAN] Trying to use layers: {:?}", unsafe {
        parse_raw_strings(&layer_names)
    });
    let extensions = vec![(vk::ExtDebugUtilsFn::name().to_owned())];

    let extension_names = extensions.iter().map(|e| e.as_ptr()).collect::<Vec<_>>();

    let app_info = vk::ApplicationInfo::builder()
        .application_name(&app_name)
        .api_version(vk::make_api_version(0, 1, 2, 0));
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
    system: xr::SystemId,
    vulkan_instance: &AshInstance,
    physical_device: vk::PhysicalDevice,
) -> Result<(Device, vk::Queue, u32)> {
    println!("[HOTHAM_VULKAN] Creating logical device.. ");

    let extension_names = xr_instance.vulkan_legacy_device_extensions(system)?;
    let mut extension_names = extension_names
        .split(' ')
        .map(|x| CString::new(x).unwrap())
        .collect::<Vec<_>>();

    add_device_extension_names(&mut extension_names);
    create_vulkan_device(&extension_names, vulkan_instance, physical_device)
}

fn create_vulkan_device(
    extension_names: &[std::ffi::CString],
    vulkan_instance: &AshInstance,
    physical_device: vk::PhysicalDevice,
) -> Result<(Device, vk::Queue, u32)> {
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

    let queue_create_info = vk::DeviceQueueCreateInfo::builder()
        .queue_priorities(&queue_priorities)
        .queue_family_index(graphics_family_index)
        .build();

    // We use a *whole bunch* of different features, and somewhat annoyingly they're all enabled in different ways.
    let enabled_features = vk::PhysicalDeviceFeatures::builder()
        .multi_draw_indirect(true)
        .sampler_anisotropy(true)
        .build();

    let mut physical_device_features = vk::PhysicalDeviceVulkan11Features::builder()
        .multiview(true)
        .shader_draw_parameters(true);

    let mut descriptor_indexing_features = vk::PhysicalDeviceDescriptorIndexingFeatures::builder()
        .shader_sampled_image_array_non_uniform_indexing(true)
        .descriptor_binding_partially_bound(true)
        .descriptor_binding_variable_descriptor_count(true)
        .descriptor_binding_sampled_image_update_after_bind(true)
        .runtime_descriptor_array(true);

    let mut robust_features =
        vk::PhysicalDeviceRobustness2FeaturesEXT::builder().null_descriptor(true);

    let device_create_info = vk::DeviceCreateInfo::builder()
        .queue_create_infos(slice_from_ref(&queue_create_info))
        .enabled_extension_names(&extension_names)
        .enabled_features(&enabled_features)
        .push_next(&mut physical_device_features)
        .push_next(&mut descriptor_indexing_features)
        .push_next(&mut robust_features);

    let device =
        unsafe { vulkan_instance.create_device(physical_device, &device_create_info, None) }?;

    let graphics_queue = unsafe { device.get_device_queue(graphics_family_index, 0) };

    println!("[HOTHAM_VULKAN] ..done");

    Ok((device, graphics_queue, graphics_family_index))
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
    } else if old_layout == vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL
        && new_layout == vk::ImageLayout::TRANSFER_SRC_OPTIMAL
    {
        return (
            vk::AccessFlags::empty(),
            vk::AccessFlags::TRANSFER_READ,
            vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            vk::PipelineStageFlags::TRANSFER,
        );
    }

    panic!("Invalid layout transition!");
}

pub fn get_test_physical_device(instance: &AshInstance) -> vk::PhysicalDevice {
    unsafe {
        println!("[HOTHAM_VULKAN] Getting physical device..");
        let devices = instance.enumerate_physical_devices().unwrap();
        devices[0]
    }
}

pub const EMPTY_KTX: [u8; 104] = [
    0xAB, 0x4B, 0x54, 0x58, 0x20, 0x31, 0x31, 0xBB, 0x0D, 0x0A, 0x1A, 0x0A, 0x01, 0x02, 0x03, 0x04,
    0x01, 0x14, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x08, 0x19, 0x00, 0x00, 0x58, 0x80, 0x00, 0x00,
    0x08, 0x19, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00,
    0x1B, 0x00, 0x00, 0x00, 0x4B, 0x54, 0x58, 0x4F, 0x72, 0x69, 0x65, 0x6E, 0x74, 0x61, 0x74, 0x69,
    0x6F, 0x6E, 0x00, 0x53, 0x3D, 0x72, 0x2C, 0x54, 0x3D, 0x64, 0x2C, 0x52, 0x3D, 0x69, 0x00, 0x00,
    0x04, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF,
];

#[cfg(test)]
mod tests {
    use super::vulkan_init_test;

    #[test]
    pub fn test_vulkan_init_smoke_test() {
        vulkan_init_test().unwrap();
    }
}
