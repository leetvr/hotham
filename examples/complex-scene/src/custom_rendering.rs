use crate::hologram::Hologram;
use hotham::{
    components::{skin::NO_SKIN, stage, GlobalTransform, Mesh, Skin, Visible},
    contexts::{
        render_context::{
            Instance, InstancedPrimitive, InstancedQuadricPrimitive, QuadricInstance,
        },
        RenderContext, VulkanContext,
    },
    glam::{Affine3A, Mat4},
    hecs::{With, World},
    rendering::resources::{DrawData, PrimitiveCullData, QuadricData, ShaderIndex},
    vk, xr, Engine,
};

/// Rendering system
/// Walks through each Mesh that is Visible and renders it.
///
/// Advanced users may instead call [`begin`], [`draw_world`], and [`end`] manually.
pub fn custom_rendering_system(engine: &mut Engine, swapchain_image_index: usize) {
    let world = &mut engine.world;
    let vulkan_context = &mut engine.vulkan_context;
    let render_context = &mut engine.render_context;

    // Update views just before rendering.
    let views = engine.xr_context.update_views();

    custom_rendering_system_inner(
        world,
        vulkan_context,
        render_context,
        views,
        swapchain_image_index,
    );
}

pub(crate) fn custom_rendering_system_inner(
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

    for (_, (hologram, global_transform)) in
        world.query_mut::<With<(&Hologram, &GlobalTransform), &Visible>>()
    {
        let mesh_data = meshes.get(hologram.mesh_data_handle).unwrap();
        for primitive in &mesh_data.primitives {
            let key = primitive.index_buffer_offset;

            // Create a transform from this primitive's local space into gos space.
            let gos_from_local = gos_from_global * global_transform.0;

            render_context
                .quadrics_primitive_map
                .entry(key)
                .or_insert(InstancedQuadricPrimitive {
                    primitive: primitive.clone(),
                    instances: Default::default(),
                })
                .instances
                .push(QuadricInstance {
                    gos_from_local,
                    bounding_sphere: primitive.get_bounding_sphere_in_gos(&gos_from_local),
                    surface_q_in_local: hologram.hologram_data.surface_q_in_local,
                    bounds_q_in_local: hologram.hologram_data.bounds_q_in_local,
                    uv_from_local: hologram.hologram_data.uv_from_local,
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
                index_offset: primitive.index_buffer_offset,
                index_shader: ShaderIndex::Triangle,
                visible: false,
            });
        }
    }
    for instanced_primitive in render_context.quadrics_primitive_map.values() {
        let primitive = &instanced_primitive.primitive;
        for (instance, i) in instanced_primitive.instances.iter().zip(0u32..) {
            cull_data.push(&PrimitiveCullData {
                bounding_sphere: instance.bounding_sphere,
                index_instance: i,
                index_offset: primitive.index_buffer_offset,
                index_shader: ShaderIndex::Quadric,
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
    let quadrics_data_buffer = &mut frame.quadric_data_buffer;
    quadrics_data_buffer.clear();

    let mut current_shader = Default::default();
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
        if cull_result.index_offset != current_primitive_id
            || cull_result.index_shader != current_shader
        {
            // Don't record commands for primitives which have no instances, eg. have been culled.
            if instance_count > 0 {
                match current_shader {
                    ShaderIndex::Triangle => {
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
                    ShaderIndex::Quadric => {
                        let primitive = &render_context
                            .quadrics_primitive_map
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
                };
            }

            current_primitive_id = cull_result.index_offset;
            instance_offset += instance_count;
            instance_count = 0;
        }

        if cull_result.index_shader != current_shader {
            current_shader = cull_result.index_shader;
            instance_offset = 0;

            // Change pipeline when we start to encounter holograms
            if let ShaderIndex::Quadric = cull_result.index_shader {
                vulkan_context.device.cmd_bind_pipeline(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    render_context.quadrics_pipeline,
                )
            }
        }

        // If this primitive is visible, increase the instance count and record its draw data.
        if cull_result.visible {
            match current_shader {
                ShaderIndex::Triangle => {
                    let instanced_primitive = render_context
                        .triangles_primitive_map
                        .get(&cull_result.index_offset)
                        .unwrap();
                    let instance =
                        &instanced_primitive.instances[cull_result.index_instance as usize];
                    let draw_data = DrawData {
                        gos_from_local: instance.gos_from_local.into(),
                        local_from_gos: instance.gos_from_local.inverse().into(),
                        material_id: instanced_primitive.primitive.material_id,
                        skin_id: instance.skin_id,
                    };
                    draw_data_buffer.push(&draw_data);
                    instance_count += 1;
                }
                ShaderIndex::Quadric => {
                    let instanced_primitive = render_context
                        .quadrics_primitive_map
                        .get(&cull_result.index_offset)
                        .unwrap();
                    let instance =
                        &instanced_primitive.instances[cull_result.index_instance as usize];
                    let local_from_gos: Mat4 = instance.gos_from_local.inverse().into();
                    let quadric_data = QuadricData {
                        gos_from_local: instance.gos_from_local.into(),
                        material_id: instanced_primitive.primitive.material_id,
                        surface_q: local_from_gos.transpose()
                            * instance.surface_q_in_local
                            * local_from_gos,
                        bounds_q: local_from_gos.transpose()
                            * instance.bounds_q_in_local
                            * local_from_gos,
                        uv_from_gos: instance.uv_from_local * local_from_gos,
                    };
                    quadrics_data_buffer.push(&quadric_data);
                    instance_count += 1;
                }
            };
        }
    }

    // Finally, record the last primitive. This is counterintuitive at first glance, but the loop above only
    // records a command when the primitive has changed. If we don't do this, the last primitive will never
    // be drawn.
    if instance_count > 0 {
        match current_shader {
            ShaderIndex::Triangle => {
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
            ShaderIndex::Quadric => {
                let primitive = &render_context
                    .quadrics_primitive_map
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
        };
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
    render_context.quadrics_primitive_map.clear();
    render_context.end_pbr_render_pass(vulkan_context);
}
