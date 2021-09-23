use crate::{
    components::{Mesh, TransformMatrix},
    resources::VulkanContext,
    resources::{render_context::create_push_constant, RenderContext},
};
use ash::vk;
use legion::system;

#[system(for_each)]
pub(crate) fn rendering(
    mesh: &mut Mesh,
    transform_matrix: &TransformMatrix,
    #[resource] vulkan_context: &VulkanContext,
    #[resource] swapchain_image_index: &usize,
    #[resource] render_context: &RenderContext,
) -> () {
    let device = &vulkan_context.device;
    let command_buffer = render_context.frames[*swapchain_image_index].command_buffer;

    unsafe {
        mesh.ubo_data.transform = transform_matrix.0.clone();
        mesh.ubo_buffer
            .update(&vulkan_context, &[mesh.ubo_data])
            .unwrap();

        // Bind mesh descriptor sets
        device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            render_context.pipeline_layout,
            2,
            &mesh.descriptor_sets,
            &[],
        );

        for primitive in &mesh.primitives {
            // Bind vertex and index buffers
            device.cmd_bind_vertex_buffers(
                command_buffer,
                0,
                &[primitive.vertex_buffer.handle],
                &[0],
            );
            device.cmd_bind_index_buffer(
                command_buffer,
                primitive.index_buffer.handle,
                0,
                vk::IndexType::UINT32,
            );

            // Bind texture descriptor sets
            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                render_context.pipeline_layout,
                1,
                &[primitive.texture_descriptor_set],
                &[],
            );

            // Push constants
            let material_push_constant = create_push_constant(&primitive.material);
            device.cmd_push_constants(
                command_buffer,
                render_context.pipeline_layout,
                vk::ShaderStageFlags::FRAGMENT,
                0,
                material_push_constant,
            );
            device.cmd_draw_indexed(command_buffer, primitive.indicies_count, 1, 0, 0, 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ash::vk::Handle;
    use image::{jpeg::JpegEncoder, DynamicImage, RgbaImage};
    use legion::{Resources, Schedule, World};
    use nalgebra::UnitQuaternion;
    use openxr::{Fovf, Quaternionf, Vector3f};

    use crate::{
        buffer::Buffer,
        gltf_loader,
        resources::{RenderContext, XrContext},
        scene_data::SceneParams,
        swapchain::Swapchain,
        systems::{update_parent_transform_matrix_system, update_transform_matrix_system},
        util::get_from_device_memory,
        COLOR_FORMAT,
    };

    #[test]
    pub fn test_rendering_system() {
        let mut world = World::default();
        let (xr_context, vulkan_context) = XrContext::new().unwrap();
        let render_context = RenderContext::new(&vulkan_context, &xr_context).unwrap();

        let mut schedule = Schedule::builder().add_system(rendering_system()).build();
        let mut resources = Resources::default();
        resources.insert(vulkan_context);
        resources.insert(render_context);
        resources.insert(0 as usize);
        schedule.execute(&mut world, &mut resources);

        let mut frame_index = resources.get_mut::<usize>().unwrap();
        (*frame_index) = 1;
        drop(frame_index);
        schedule.execute(&mut world, &mut resources);
    }

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

        let render_context =
            RenderContext::new_from_swapchain(&vulkan_context, &swapchain).unwrap();

        // Get a model from GLTF
        // let gltf_data: Vec<(&[u8], &[u8])> = vec![(
        //     include_bytes!("../../../test_assets/Sponza.gltf"),
        //     include_bytes!("../../../test_assets/Sponza.bin"),
        // )];
        let gltf_data: Vec<(&[u8], &[u8])> = vec![(
            include_bytes!("../../../test_assets/damaged_helmet.gltf"),
            include_bytes!("../../../test_assets/damaged_helmet_data.bin"),
        )];
        let mut models = gltf_loader::load_models_from_gltf(
            gltf_data,
            &vulkan_context,
            &render_context.descriptor_set_layouts,
        )
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
                &render_context,
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
        render_context: &RenderContext,
        world: &mut World,
        resolution: vk::Extent2D,
        image: crate::image::Image,
        name: &str,
        debug_view_equation: f32,
    ) {
        let mut schedule = Schedule::builder()
            .add_thread_local_fn(move |_, resources| {
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
                let vulkan_context = resources.get::<VulkanContext>().unwrap();
                let mut render_context = resources.get_mut::<RenderContext>().unwrap();
                render_context
                    .update_scene_data(&views, &vulkan_context)
                    .unwrap();
                render_context
                    .scene_params
                    .update(
                        &vulkan_context,
                        &[SceneParams {
                            debug_view_equation,
                            ..Default::default()
                        }],
                    )
                    .unwrap();
                render_context.begin_render_pass(&vulkan_context, 0);
            })
            .add_system(update_transform_matrix_system())
            .add_system(update_parent_transform_matrix_system())
            .add_system(rendering_system())
            .add_thread_local_fn(|_, resources| {
                let vulkan_context = resources.get::<VulkanContext>().unwrap();
                let mut render_context = resources.get_mut::<RenderContext>().unwrap();
                render_context.end_render_pass(&vulkan_context, 0);
            })
            .build();
        let mut resources = Resources::default();
        resources.insert(vulkan_context.clone());
        resources.insert(render_context.clone());
        resources.insert(0 as usize);
        schedule.execute(world, &mut resources);
        let size = (resolution.height * resolution.width * 4) as usize;
        let vulkan_context = resources.get::<VulkanContext>().unwrap();
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
        let path = format!("../test_assets/render_{}.jpeg", name);
        let path = std::path::Path::new(&path);
        let mut file = std::fs::File::create(path).unwrap();
        let mut jpeg_encoder = JpegEncoder::new(&mut file);
        jpeg_encoder.encode_image(&image_from_vulkan).unwrap();
    }
}
