use std::convert::TryInto;

use crate::{
    components::{skin::NO_SKIN, GlobalTransform, Mesh, Skin, Visible},
    rendering::resources::{DrawData, PrimitiveCullData},
    resources::VulkanContext,
    resources::{
        render_context::{Instance, InstancedPrimitive},
        RenderContext,
    },
};
use hecs::{PreparedQuery, With, World};
use openxr as xr;

/// Rendering system
/// Walks through each Mesh that is Visible and renders it.
///
/// Requirements:
/// - BEFORE: ensure you have called render_context.begin_frame
/// - AFTER: ensure you have called render_context.end_frame
///
/// Advanced users may instead call [`begin`], [`draw_world`], and [`end`] manually.
#[allow(clippy::type_complexity)]
pub fn rendering_system(
    query: &mut PreparedQuery<With<Visible, (&Mesh, &GlobalTransform, Option<&Skin>)>>,
    world: &mut World,
    vulkan_context: &VulkanContext,
    render_context: &mut RenderContext,
    views: &[xr::View],
    swapchain_image_index: usize,
) {
    unsafe {
        begin(
            query,
            world,
            vulkan_context,
            render_context,
            views,
            swapchain_image_index,
        );
        draw_world(vulkan_context, render_context);
        end(vulkan_context, render_context);
    }
}

/// Prepare to draw the world
///
/// Begins the render pass used to draw the world, but records no drawing commands.
///
/// # Safety
///
/// Must be called at the start of the process or after [`end`]
#[allow(clippy::type_complexity)]
pub unsafe fn begin(
    query: &mut PreparedQuery<With<Visible, (&Mesh, &GlobalTransform, Option<&Skin>)>>,
    world: &mut World,
    vulkan_context: &VulkanContext,
    render_context: &mut RenderContext,
    views: &[xr::View],
    swapchain_image_index: usize,
) {
    // First, we need to walk through each entity that contains a mesh, collect its primitives
    // and create a list of instances, indexed by primitive ID.
    //
    // We use primitive.index_buffer_offset as our primitive ID as it is guaranteed to be unique between
    // primitives.
    let meshes = &render_context.resources.mesh_data;

    for (_, (mesh, global_transform, skin)) in query.query_mut(world) {
        let mesh = meshes.get(mesh.handle).unwrap();
        let skin_id = skin.map(|s| s.id).unwrap_or(NO_SKIN);
        for primitive in &mesh.primitives {
            let key = primitive.index_buffer_offset;

            render_context
                .primitive_map
                .entry(key)
                .or_insert(InstancedPrimitive {
                    primitive: primitive.clone(),
                    instances: Default::default(),
                })
                .instances
                .push(Instance {
                    global_from_local: global_transform.0,
                    bounding_sphere: primitive.get_bounding_sphere(global_transform),
                    skin_id,
                });
        }
    }

    // Next organize this data into a layout that's easily consumed by the compute shader.
    // ORDER IS IMPORTANT HERE! The final buffer should look something like:
    //
    // primitive_a
    // primitive_a
    // primitive_c
    // primitive_b
    // primitive_b
    // primitive_e
    // primitive_e
    //
    // ..etc. The most important thing is that each instances are grouped by their primitive.
    let frame = &mut render_context.frames[render_context.frame_index];
    let cull_data = &mut frame.primitive_cull_data_buffer;
    cull_data.clear();

    for instanced_primitive in render_context.primitive_map.values() {
        let primitive = &instanced_primitive.primitive;
        for (i, instance) in instanced_primitive.instances.iter().enumerate() {
            cull_data.push(&PrimitiveCullData {
                index_instance: i.try_into().unwrap(),
                index_offset: primitive.index_buffer_offset,
                bounding_sphere: instance.bounding_sphere,
                visible: false,
            });
        }
    }

    // This is the VERY LATEST we can possibly update our views, as the compute shader will need them.
    render_context.update_scene_data(views);

    // Execute the culling shader on the GPU.
    render_context.cull_objects(vulkan_context);

    // Begin the render pass, bind descriptor sets.
    render_context.begin_pbr_render_pass(vulkan_context, swapchain_image_index);
}

/// Draw the world
///
/// Records commands to draw all visible meshes
///
/// # Safety
///
/// Must be between [`begin`] and [`end`]
pub unsafe fn draw_world(vulkan_context: &VulkanContext, render_context: &mut RenderContext) {
    // Parse through the cull buffer and record commands. This is a bit complex.
    let device = &vulkan_context.device;
    let frame = &mut render_context.frames[render_context.frame_index];
    let command_buffer = frame.command_buffer;
    let draw_data_buffer = &mut frame.draw_data_buffer;
    draw_data_buffer.clear();

    let mut instance_offset = 0;
    let mut current_primitive_id = u32::MAX;
    let mut instance_count = 0;
    let cull_data = frame.primitive_cull_data_buffer.as_slice();

    for cull_result in cull_data {
        // If we haven't yet set our primitive ID, set it now.
        if current_primitive_id == u32::MAX {
            current_primitive_id = cull_result.index_offset;
        }

        // We're finished with this primitive. Record the command and increase our offset.
        if cull_result.index_offset != current_primitive_id {
            let primitive = &render_context
                .primitive_map
                .get(&current_primitive_id)
                .unwrap()
                .primitive;

            // Don't record commands for primitives which have no instances, eg. have been culled.
            if instance_count > 0 {
                device.cmd_draw_indexed(
                    command_buffer,
                    primitive.indices_count,
                    instance_count,
                    primitive.index_buffer_offset,
                    primitive.vertex_buffer_offset as _,
                    instance_offset,
                );
            }

            current_primitive_id = cull_result.index_offset;
            instance_offset += instance_count;
            instance_count = 0;
        }

        // If this primitive is visible, increase the instance count and record its draw data.
        if cull_result.visible {
            let instanced_primitive = render_context
                .primitive_map
                .get(&current_primitive_id)
                .unwrap();
            let instance = &instanced_primitive.instances[cull_result.index_instance as usize];
            let draw_data = DrawData {
                global_from_local: instance.global_from_local,
                inverse_transpose: instance
                    .global_from_local
                    .try_inverse()
                    .unwrap()
                    .transpose(),
                material_id: instanced_primitive.primitive.material_id,
                skin_id: instance.skin_id,
            };
            draw_data_buffer.push(&draw_data);
            instance_count += 1;
        }
    }

    // Finally, record the last primitive. This is counterintuitive at first glance, but the loop above only
    // records a command when the primitive has changed. If we don't do this, the last primitive will never
    // be drawn.
    if instance_count > 0 {
        let primitive = &render_context
            .primitive_map
            .get(&current_primitive_id)
            .unwrap()
            .primitive;
        device.cmd_draw_indexed(
            command_buffer,
            primitive.indices_count,
            instance_count,
            primitive.index_buffer_offset,
            primitive.vertex_buffer_offset as _,
            instance_offset,
        );
    }
}

/// Finish drawing
///
/// # Safety
///
/// Must be called after `begin`
pub fn end(vulkan_context: &VulkanContext, render_context: &mut RenderContext) {
    // OK. We're all done!
    render_context.primitive_map.clear();
    render_context.end_pbr_render_pass(vulkan_context);
}

#[cfg(target_os = "windows")]
#[cfg(test)]
mod tests {
    #![allow(deprecated)]
    use std::{collections::hash_map::DefaultHasher, hash::Hasher};

    use super::*;
    use ash::vk;
    use ash::vk::Handle;
    use image::{codecs::jpeg::JpegEncoder, DynamicImage, RgbaImage};
    use nalgebra::UnitQuaternion;
    use openxr::{Fovf, Quaternionf, Vector3f};

    use crate::{
        asset_importer,
        rendering::{image::Image, legacy_buffer::Buffer, scene_data, swapchain::SwapchainInfo},
        resources::RenderContext,
        systems::{update_global_transform_system, update_global_transform_with_parent_system},
        util::get_from_device_memory,
        COLOR_FORMAT,
    };

    #[test]
    pub fn test_rendering_pbr() {
        let vulkan_context = VulkanContext::testing().unwrap();
        let resolution = vk::Extent2D {
            height: 800,
            width: 800,
        };
        // Create an image with vulkan_context
        let image = vulkan_context
            .create_image(
                COLOR_FORMAT,
                &resolution,
                vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC,
                2,
                1,
            )
            .unwrap();
        vulkan_context
            .set_debug_name(vk::ObjectType::IMAGE, image.handle.as_raw(), "Screenshot")
            .unwrap();

        let swapchain = SwapchainInfo {
            images: vec![image.handle],
            resolution,
        };

        let mut render_context =
            RenderContext::new_from_swapchain_info(&vulkan_context, &swapchain).unwrap();

        let gltf_data: Vec<&[u8]> = vec![include_bytes!("../../../test_assets/damaged_helmet.glb")];
        let mut models =
            asset_importer::load_models_from_glb(&gltf_data, &vulkan_context, &mut render_context)
                .unwrap();
        let (_, mut world) = models.drain().next().unwrap();
        let params = vec![
            ("Full", 0.0, scene_data::DEFAULT_IBL_INTENSITY),
            ("Diffuse", 1.0, scene_data::DEFAULT_IBL_INTENSITY),
            ("Normals", 2.0, scene_data::DEFAULT_IBL_INTENSITY),
            ("No_IBL", 0.0, 0.0),
        ];

        let errors: Vec<_> = params
            .iter()
            .filter_map(|(name, debug_shader_inputs, debug_ibl_intensity)| {
                render_object_with_debug_data(
                    &vulkan_context,
                    &mut render_context,
                    &mut world,
                    resolution,
                    image.clone(),
                    name,
                    *debug_shader_inputs,
                    *debug_ibl_intensity,
                )
                .err()
            })
            .collect();
        assert!(errors.is_empty(), "{:#?}", errors);
    }

    fn render_object_with_debug_data(
        vulkan_context: &VulkanContext,
        render_context: &mut RenderContext,
        world: &mut World,
        resolution: vk::Extent2D,
        image: Image,
        name: &str,
        debug_shader_inputs: f32,
        debug_ibl_intensity: f32,
    ) -> Result<(), String> {
        // Render the scene
        let mut renderdoc = begin_renderdoc();
        render(
            render_context,
            vulkan_context,
            debug_shader_inputs,
            debug_ibl_intensity,
            world,
        );
        if let Ok(renderdoc) = renderdoc.as_mut() {
            end_renderdoc(renderdoc);
        }

        // Save the resulting image to the disk and get its hash, along with a "known good" hash
        // of what the image *should* be.
        save_image_to_disk(resolution, vulkan_context, image, name)
    }

    fn save_image_to_disk(
        resolution: vk::Extent2D,
        vulkan_context: &VulkanContext,
        image: Image,
        name: &str,
    ) -> Result<(), String> {
        let size = (resolution.height * resolution.width * 4) as usize;
        let image_data = vec![0; size];
        let buffer = Buffer::new(
            &vulkan_context,
            &image_data,
            vk::BufferUsageFlags::TRANSFER_DST,
        )
        .unwrap();
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
            buffer.handle,
        );
        let image_bytes = unsafe { get_from_device_memory(&vulkan_context, &buffer) }.to_vec();
        let image_from_vulkan = DynamicImage::ImageRgba8(
            RgbaImage::from_raw(resolution.width, resolution.height, image_bytes).unwrap(),
        );
        let output_path = format!("../test_assets/render_{}.jpg", name);
        {
            let output_path = std::path::Path::new(&output_path);
            let mut file = std::fs::File::create(output_path).unwrap();
            let mut jpeg_encoder = JpegEncoder::new(&mut file);
            jpeg_encoder.encode_image(&image_from_vulkan).unwrap();
        }
        let output_hash = hash_file(&output_path);
        let known_good_path = format!("../test_assets/render_{}_known_good.jpg", name);
        let known_good_hash = hash_file(&known_good_path);

        if !output_hash.is_ok() {
            return Err(format!("Failed to hash output image: {}", name));
        }
        if !known_good_hash.is_ok() {
            return Err(format!("Failed to hash known good image: {}", name));
        }
        if output_hash != known_good_hash {
            return Err(format!("Bad render: {}", name));
        }
        Ok(())
    }

    fn render(
        render_context: &mut RenderContext,
        vulkan_context: &VulkanContext,
        debug_shader_inputs: f32,
        debug_ibl_intensity: f32,
        world: &mut World,
    ) {
        // HELMET
        let rotation: mint::Quaternion<f32> =
            UnitQuaternion::from_euler_angles(0., 45_f32.to_radians(), 0.).into();
        let position = Vector3f {
            x: 1.4,
            y: 0.0,
            z: 1.4,
        };
        let view = openxr::View {
            pose: openxr::Posef {
                orientation: Quaternionf::from(rotation),
                position,
            },
            fov: Fovf {
                angle_up: 45.0_f32.to_radians(),
                angle_down: -45.0_f32.to_radians(),
                angle_left: -45.0_f32.to_radians(),
                angle_right: 45.0_f32.to_radians(),
            },
        };
        let views = vec![view.clone(), view];
        render_context.begin_frame(vulkan_context);
        render_context.scene_data.params.z = debug_shader_inputs;
        render_context.scene_data.params.x = debug_ibl_intensity;
        update_global_transform_system(&mut Default::default(), world);
        update_global_transform_with_parent_system(
            &mut Default::default(),
            &mut Default::default(),
            world,
        );
        rendering_system(
            &mut Default::default(),
            world,
            vulkan_context,
            render_context,
            &views,
            0,
        );
        render_context.end_frame(vulkan_context);
    }

    fn hash_file(file_path: &str) -> Result<u64, ()> {
        let mut hasher = DefaultHasher::new();
        let bytes = match std::fs::read(&file_path) {
            Ok(it) => it,
            Err(_) => return Err(()),
        };
        bytes.iter().for_each(|b| hasher.write_u8(*b));
        return Ok(hasher.finish());
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    use renderdoc::RenderDoc;

    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    fn begin_renderdoc() -> Result<RenderDoc<renderdoc::V141>, renderdoc::Error> {
        let mut renderdoc = RenderDoc::<renderdoc::V141>::new()?;
        renderdoc.start_frame_capture(std::ptr::null(), std::ptr::null());
        Ok(renderdoc)
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    fn end_renderdoc(renderdoc: &mut RenderDoc<renderdoc::V141>) {
        let _ = renderdoc.end_frame_capture(std::ptr::null(), std::ptr::null());
    }
}
