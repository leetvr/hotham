use crate::{
    components::{Mesh, TransformMatrix, Visible},
    resources::VulkanContext,
    resources::{render_context::create_push_constant, RenderContext},
};
use ash::vk;
use hecs::{PreparedQuery, With, World};

/// Rendering system
/// Walks through each Mesh that is Visible and renders it.
pub fn rendering_system(
    query: &mut PreparedQuery<With<Visible, (&mut Mesh, &TransformMatrix)>>,
    world: &mut World,
    vulkan_context: &VulkanContext,
    swapchain_image_index: usize,
    render_context: &RenderContext,
) {
    for (_, (mesh, transform_matrix)) in query.query_mut(world) {
        let device = &vulkan_context.device;
        let command_buffer = render_context.frames[swapchain_image_index].command_buffer;

        unsafe {
            // for primitive in &mesh.primitives {
            //     // Bind vertex and index buffers
            //     device.cmd_bind_vertex_buffers(
            //         command_buffer,
            //         0,
            //         &[primitive.vertex_buffer.handle],
            //         &[0],
            //     );
            //     device.cmd_bind_index_buffer(
            //         command_buffer,
            //         primitive.index_buffer.handle,
            //         0,
            //         vk::IndexType::UINT32,
            //     );

            //     // Bind texture descriptor sets
            //     device.cmd_bind_descriptor_sets(
            //         command_buffer,
            //         vk::PipelineBindPoint::GRAPHICS,
            //         render_context.pipeline_layout,
            //         1,
            //         &[primitive.texture_descriptor_set],
            //         &[],
            //     );

            //     // Push constants
            //     let material_push_constant = create_push_constant(&primitive.material);
            //     device.cmd_push_constants(
            //         command_buffer,
            //         render_context.pipeline_layout,
            //         vk::ShaderStageFlags::FRAGMENT,
            //         0,
            //         material_push_constant,
            //     );
            //     device.cmd_draw_indexed(command_buffer, primitive.indices_count, 1, 0, 0, 1);
            // }
        }
    }
}

#[cfg(target_os = "windows")]
#[cfg(test)]
mod tests {
    use std::{collections::hash_map::DefaultHasher, hash::Hasher};

    use super::*;
    use ash::vk::Handle;
    use image::{jpeg::JpegEncoder, DynamicImage, RgbaImage};
    use nalgebra::UnitQuaternion;
    use openxr::{Fovf, Quaternionf, Vector3f};

    use crate::{
        asset_importer,
        rendering::{buffer::Buffer, image::Image, scene_data::SceneParams, swapchain::Swapchain},
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

        let swapchain = Swapchain {
            images: vec![image.handle],
            resolution,
        };

        let mut render_context =
            RenderContext::new_from_swapchain(&vulkan_context, &swapchain).unwrap();

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
        render(render_context, vulkan_context, debug_view_equation, world);

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

        //  TODO: Fix this on non-windows platforms.
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
        render_context
            .update_scene_data(&views, &vulkan_context)
            .unwrap();
        render_context.begin_frame(&vulkan_context, 0);
        render_context.begin_pbr_render_pass(&vulkan_context, 0);
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
            0,
            render_context,
        );
        render_context.end_pbr_render_pass(&vulkan_context, 0);
        render_context.end_frame(&vulkan_context, 0);
    }

    fn hash_file(file_path: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        let bytes = std::fs::read(&file_path).unwrap();
        bytes.iter().for_each(|b| hasher.write_u8(*b));
        return hasher.finish();
    }
}
