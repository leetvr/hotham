use crate::{
    components::{skin::NO_SKIN, stage, GlobalTransform, Mesh, Skin, Visible},
    contexts::VulkanContext,
    contexts::{
        render_context::{Instance, InstancedPrimitive},
        RenderContext,
    },
    rendering::resources::{DrawData, PrimitiveCullData},
    Engine,
};
use glam::Affine3A;
use hecs::{With, World};
use openxr as xr;

/// Rendering system
/// Walks through each Mesh that is Visible and renders it.
///
/// Advanced users may instead call [`begin`], [`draw_world`], and [`end`] manually.
pub fn rendering_system(engine: &mut Engine, swapchain_image_index: usize) {
    let world = &mut engine.world;
    let vulkan_context = &mut engine.vulkan_context;
    let render_context = &mut engine.render_context;

    // Update views just before rendering.
    let views = engine.xr_context.update_views();

    rendering_system_inner(
        world,
        vulkan_context,
        render_context,
        views,
        swapchain_image_index,
    );
}

pub(crate) fn rendering_system_inner(
    world: &mut World,
    vulkan_context: &VulkanContext,
    render_context: &mut RenderContext,
    views: &[xr::View],
    swapchain_image_index: usize,
) {
    unsafe {
        begin(
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

    // Create transformations to globally oriented stage space
    let global_from_stage = stage::get_global_from_stage(world);

    // `gos_from_global` is just the inverse of `global_from_stage`'s translation - rotation is ignored.
    let gos_from_global =
        Affine3A::from_translation(global_from_stage.translation.into()).inverse();

    let gos_from_stage: Affine3A = gos_from_global * global_from_stage;

    for (_, (mesh, global_transform, skin)) in
        world.query_mut::<With<(&Mesh, &GlobalTransform, Option<&Skin>), &Visible>>()
    {
        let mesh = meshes.get(mesh.handle).unwrap();
        let skin_id = skin.map(|s| s.id).unwrap_or(NO_SKIN);
        for primitive in &mesh.primitives {
            let key = primitive.index_buffer_offset;

            // Create a transform from this primitive's local space into gos space.
            let gos_from_local = gos_from_global * global_transform.0;

            render_context
                .triangles_primitive_map
                .entry(key)
                .or_insert(InstancedPrimitive {
                    primitive: primitive.clone(),
                    instances: Default::default(),
                })
                .instances
                .push(Instance {
                    gos_from_local,
                    bounding_sphere: primitive.get_bounding_sphere_in_gos(&gos_from_local),
                    skin_id,
                });
        }
    }

    // Next organize this data into a layout that's easily consumed by the compute shader.
    // ORDER IS IMPORTANT HERE! The final buffer should look something like:
    //
    // triangles_primitive_a
    // triangles_primitive_a
    // triangles_primitive_c
    // triangles_primitive_b
    // triangles_primitive_b
    // triangles_primitive_e
    // triangles_primitive_e
    // quadrics_primitive_h
    // quadrics_primitive_g
    // quadrics_primitive_g
    // quadrics_primitive_f
    //
    // ..etc. The most important thing is that each instances are grouped by their type and then by their primitive.
    let frame = &mut render_context.frames[render_context.frame_index];
    let cull_data = &mut frame.primitive_cull_data_buffer;
    cull_data.clear();

    for instanced_primitive in render_context.triangles_primitive_map.values() {
        let primitive = &instanced_primitive.primitive;
        for (instance, i) in instanced_primitive.instances.iter().zip(0u32..) {
            cull_data.push(&PrimitiveCullData {
                bounding_sphere: instance.bounding_sphere,
                index_instance: i,
                primitive_id: primitive.index_buffer_offset,
                visible: false,
            });
        }
    }

    // This is the VERY LATEST we can possibly update our views, as the compute shader will need them.
    render_context.update_scene_data(views, &gos_from_global, &gos_from_stage);

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
            current_primitive_id = cull_result.primitive_id;
        }

        // We're finished with this primitive. Record the command and increase our offset.
        if cull_result.primitive_id != current_primitive_id {
            // Don't record commands for primitives which have no instances, eg. have been culled.
            if instance_count > 0 {
                let primitive = &render_context
                    .triangles_primitive_map
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

            current_primitive_id = cull_result.primitive_id;
            instance_offset += instance_count;
            instance_count = 0;
        }

        // If this primitive is visible, increase the instance count and record its draw data.
        if cull_result.visible {
            let instanced_primitive = render_context
                .triangles_primitive_map
                .get(&cull_result.primitive_id)
                .unwrap();
            let instance = &instanced_primitive.instances[cull_result.index_instance as usize];
            let draw_data = DrawData {
                gos_from_local: instance.gos_from_local.into(),
                local_from_gos: instance.gos_from_local.inverse().into(),
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
            .triangles_primitive_map
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
    render_context.triangles_primitive_map.clear();
    render_context.end_pbr_render_pass(vulkan_context);
}

#[cfg(target_os = "windows")]
#[cfg(test)]
mod tests {
    use super::*;
    use openxr::{Fovf, Quaternionf, Vector3f};

    use crate::{
        asset_importer,
        components::{stage::Stage, LocalTransform},
        contexts::RenderContext,
        rendering::{image::Image, light::Light, scene_data},
        systems::{
            update_global_transform::update_global_transform_system_inner,
            update_global_transform_with_parent::update_global_transform_with_parent_system_inner,
        },
        util::{affine_from_posef, posef_from_affine, save_image_to_disk},
    };
    use glam::{Quat, Vec3};

    #[test]
    pub fn test_rendering_pbr() {
        let (mut render_context, vulkan_context, image) = RenderContext::testing_with_image();

        let gltf_data: Vec<&[u8]> = vec![include_bytes!("../../../test_assets/damaged_helmet.glb")];
        let mut models =
            asset_importer::load_models_from_glb(&gltf_data, &vulkan_context, &mut render_context)
                .unwrap();
        let (_, mut world) = models.drain().next().unwrap();

        // Add stage transform
        let stage_local_transform = LocalTransform {
            translation: [0.1, 0.2, 0.3].into(),
            rotation: Quat::from_scaled_axis(Vec3::Y * (std::f32::consts::TAU * 0.1)),
            ..Default::default()
        };
        let global_from_stage = stage_local_transform.to_affine();
        world.spawn((
            Stage {},
            stage_local_transform,
            GlobalTransform(global_from_stage.clone()),
        ));

        // Set views
        let rotation: mint::Quaternion<f32> =
            Quat::from_axis_angle(Vec3::Y, 45_f32.to_radians()).into();
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
        let mut views = vec![view.clone(), view];

        // Compensate stage transform by adjusting the views
        for view in &mut views {
            view.pose =
                posef_from_affine(global_from_stage.inverse() * affine_from_posef(view.pose));
        }

        let params = vec![
            (
                "Full",
                0.0,
                scene_data::DEFAULT_IBL_INTENSITY,
                Light::none(),
            ),
            (
                "Diffuse",
                1.0,
                scene_data::DEFAULT_IBL_INTENSITY,
                Light::none(),
            ),
            (
                "Normals",
                2.0,
                scene_data::DEFAULT_IBL_INTENSITY,
                Light::none(),
            ),
            ("No_IBL", 0.0, 0.0, Light::none()),
            (
                "Spotlight",
                0.0,
                0.0,
                Light::new_spotlight(
                    [-1., -0.1, 0.2].into(),
                    10.,
                    5.,
                    [1., 1., 1.].into(),
                    [2., 0.2, -0.4].into(),
                    0.,
                    0.3,
                ),
            ),
        ];

        let errors: Vec<_> = params
            .iter()
            .filter_map(|(name, debug_shader_inputs, debug_ibl_intensity, light)| {
                render_object_with_debug_data(
                    &vulkan_context,
                    &mut render_context,
                    &mut world,
                    image.clone(),
                    name,
                    *debug_shader_inputs,
                    *debug_ibl_intensity,
                    light,
                    &views,
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
        image: Image,
        name: &str,
        debug_shader_inputs: f32,
        debug_ibl_intensity: f32,
        light: &Light,
        views: &Vec<openxr::View>,
    ) -> Result<(), String> {
        // Render the scene

        // If you want to debug with renderdoc, uncomment the line below:
        // let mut renderdoc = begin_renderdoc();

        render(
            render_context,
            vulkan_context,
            debug_shader_inputs,
            debug_ibl_intensity,
            world,
            light,
            views,
        );

        // if let Ok(renderdoc) = renderdoc.as_mut() {
        //     end_renderdoc(renderdoc);
        // }

        // Save the resulting image to the disk and get its hash, along with a "known good" hash
        // of what the image *should* be.
        unsafe { save_image_to_disk(vulkan_context, image, name) }
    }

    fn render(
        render_context: &mut RenderContext,
        vulkan_context: &VulkanContext,
        debug_shader_inputs: f32,
        debug_ibl_intensity: f32,
        world: &mut World,
        light: &Light,
        views: &Vec<openxr::View>,
    ) {
        render_context.begin_frame(vulkan_context);
        render_context.scene_data.params.z = debug_shader_inputs;
        render_context.scene_data.params.x = debug_ibl_intensity;
        render_context.scene_data.lights[0] = light.clone();
        update_global_transform_system_inner(world);
        update_global_transform_with_parent_system_inner(world);
        rendering_system_inner(world, vulkan_context, render_context, views, 0);
        render_context.end_frame(vulkan_context);
    }
}
