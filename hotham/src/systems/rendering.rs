use std::collections::HashMap;

use crate::{
    components::{skin::NO_SKIN, Mesh, Skin, Transform, TransformMatrix, Visible},
    rendering::{
        primitive::Primitive,
        resources::{DrawData, PrimitiveCullData},
    },
    resources::RenderContext,
    resources::VulkanContext,
};
use hecs::{PreparedQuery, With, World};
use nalgebra::{Matrix4, Vector4};
use openxr as xr;

struct Instance {
    transform_matrix: Matrix4<f32>,
    bounding_sphere: Vector4<f32>,
    skin_id: u32,
}

struct InstancedPrimitive {
    primitive: Primitive,
    instances: Vec<Instance>,
}

/// Rendering system
/// Walks through each Mesh that is Visible and renders it.
///
/// Requirements:
/// - BEFORE: ensure you have called render_context.begin_frame
/// - AFTER: ensure you have called render_context.end_frame
#[allow(clippy::type_complexity)]
pub fn rendering_system(
    query: &mut PreparedQuery<With<Visible, (&Mesh, &Transform, &TransformMatrix, Option<&Skin>)>>,
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
    let mut primitive_map: HashMap<u32, InstancedPrimitive> = Default::default();
    let meshes = &render_context.resources.mesh_data;

    for (_, (mesh, transform, transform_matrix, skin)) in query.query_mut(world) {
        let mesh = meshes.get(mesh.handle).unwrap();
        let skin_id = skin.map(|s| s.id).unwrap_or(NO_SKIN);
        for primitive in &mesh.primitives {
            let key = primitive.index_buffer_offset;

            primitive_map
                .entry(key)
                .or_insert(InstancedPrimitive {
                    primitive: primitive.clone(),
                    instances: Default::default(),
                })
                .instances
                .push(Instance {
                    transform_matrix: transform_matrix.0,
                    bounding_sphere: primitive.get_bounding_sphere(transform),
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

    for instanced_primitive in primitive_map.values() {
        let primitive = &instanced_primitive.primitive;
        for instance in &instanced_primitive.instances {
            unsafe {
                cull_data.push(&PrimitiveCullData {
                    draw_data: DrawData {
                        transform: instance.transform_matrix,
                        inverse_transpose: instance
                            .transform_matrix
                            .try_inverse()
                            .unwrap()
                            .transpose(),
                        material_id: primitive.material_id,
                        skin_id: instance.skin_id,
                    },
                    index_offset: primitive.index_buffer_offset,
                    bounding_sphere: instance.bounding_sphere,
                    visible: true,
                });
            }
        }
    }

    // This is the VERY LATEST we can possibly update our views, as the compute shader will need them.
    render_context.update_scene_data(views).unwrap();

    // Execute the culling shader on the GPU.
    // render_context.cull_objects(vulkan_context);

    // Begin the render pass, bind descriptor sets.
    render_context.begin_pbr_render_pass(vulkan_context, swapchain_image_index);

    // Parse through the cull buffer and record commands. This is a bit complex.
    unsafe {
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
                let primitive = &primitive_map.get(&current_primitive_id).unwrap().primitive;

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
                draw_data_buffer.push(&cull_result.draw_data);
                instance_count += 1;
            }
        }

        // Finally, record the last primitive. This is counterintuitive at first glance, but the loop above only
        // records a command when the primitive has changed. If we don't do this, the last primitive will never
        // be drawn.
        if instance_count > 0 {
            let primitive = &primitive_map.get(&current_primitive_id).unwrap().primitive;
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

    // OK. We're all done!
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
        rendering::{image::Image, legacy_buffer::Buffer, swapchain::SwapchainInfo},
        resources::RenderContext,
        systems::{update_parent_transform_matrix_system, update_transform_matrix_system},
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

        // Get a model from GLTF
        // let gltf_data: Vec<(&[u8], &[u8])> = vec![(
        //     include_bytes!("../../../test_assets/Sponza.gltf"),
        //     include_bytes!("../../../test_assets/Sponza.bin"),
        // )];
        let gltf_data: Vec<&[u8]> = vec![include_bytes!("../../../test_assets/damaged_helmet.glb")];
        let mut models =
            asset_importer::load_models_from_glb(&gltf_data, &vulkan_context, &mut render_context)
                .unwrap();
        let (_, mut world) = models.drain().next().unwrap();
        let params = vec![
            ("Normal", 0.0),
            ("Diffuse", 1.0),
            ("F", 2.0),
            ("G", 3.0),
            ("D", 4.0),
            ("Specular", 5.0),
        ];

        for (name, debug_view_equation) in &params {
            render_object_with_debug_equation(
                &vulkan_context,
                &mut render_context,
                &mut world,
                resolution,
                image.clone(),
                name,
                *debug_view_equation,
            );
        }
    }

    fn render_object_with_debug_equation(
        vulkan_context: &VulkanContext,
        render_context: &mut RenderContext,
        world: &mut World,
        resolution: vk::Extent2D,
        image: Image,
        name: &str,
        debug_view_equation: f32,
    ) {
        // Render the scene
        let mut renderdoc = begin_renderdoc();
        render(render_context, vulkan_context, debug_view_equation, world);
        end_renderdoc(&mut renderdoc);

        // Save the resulting image to the disk and get its hash, along with a "known good" hash
        // of what the image *should* be.
        save_image_to_disk(resolution, vulkan_context, image, name);
    }

    fn save_image_to_disk(
        resolution: vk::Extent2D,
        vulkan_context: &VulkanContext,
        image: Image,
        name: &str,
    ) {
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

        assert_eq!(output_hash, known_good_hash, "Bad render: {}", name);
    }

    fn render(
        render_context: &mut RenderContext,
        vulkan_context: &VulkanContext,
        debug_view_equation: f32,
        world: &mut World,
    ) {
        // SPONZA
        // let rotation: mint::Quaternion<f32> =
        //     UnitQuaternion::from_euler_angles(0., 90_f32.to_radians(), 0.).into();
        // let position = Vector3f {
        //     x: 0.0,
        //     y: 1.4,
        //     z: 0.0,
        // };

        // HELMET
        let rotation: mint::Quaternion<f32> =
            UnitQuaternion::from_euler_angles(0., 45_f32.to_radians(), 0.).into();
        let position = Vector3f {
            x: 0.8,
            y: 1.4,
            z: 0.8,
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
        render_context.scene_data.debug_data.y = debug_view_equation;
        update_transform_matrix_system(&mut Default::default(), world);
        update_parent_transform_matrix_system(
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

    fn hash_file(file_path: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        let bytes = std::fs::read(&file_path).unwrap();
        bytes.iter().for_each(|b| hasher.write_u8(*b));
        return hasher.finish();
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    use renderdoc::RenderDoc;

    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    fn begin_renderdoc() -> RenderDoc<renderdoc::V141> {
        let mut renderdoc = RenderDoc::<renderdoc::V141>::new().unwrap();
        renderdoc.start_frame_capture(std::ptr::null(), std::ptr::null());
        renderdoc
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    fn end_renderdoc(renderdoc: &mut RenderDoc<renderdoc::V141>) {
        let _ = renderdoc.end_frame_capture(std::ptr::null(), std::ptr::null());
    }
}
