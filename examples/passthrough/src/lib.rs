use hotham::{
    components::panel::add_panel_to_world,
    hecs::World,
    schedule_functions::{
        begin_frame, begin_pbr_renderpass, end_frame, end_pbr_renderpass, physics_step,
    },
    systems::{
        animation_system, collision_system, draw_gui_system, hands_system, pointers_system,
        rendering::rendering_system, skinning::skinning_system,
        update_parent_transform_matrix_system, update_rigid_body_transforms_system,
        update_transform_matrix_system, Queries,
    },
    Engine, HothamError, HothamResult,
};

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
pub fn main() {
    real_main().expect("Error running app!");
}

pub fn real_main() -> HothamResult<()> {
    let mut vroom = Vroom::new()?;

    vroom.run()
}

struct Vroom<'a> {
    engine: Engine,
    world: World,
    hotham_queries: Queries<'a>,
}

impl<'a> Vroom<'a> {
    pub fn new() -> Result<Self, HothamError> {
        let mut engine = Engine::new();
        let render_context = &mut engine.render_context;
        let vulkan_context = &mut engine.vulkan_context;
        let gui_context = &mut engine.gui_context;
        let physics_context = &mut engine.physics_context;
        let mut world = World::default();
        add_panel_to_world(
            "This panel should be displayed on top of the real world",
            1600,
            900,
            vec![],
            [0., 1.5, -2.].into(),
            vulkan_context,
            render_context,
            gui_context,
            physics_context,
            &mut world,
        );

        Ok(Self {
            engine,
            world,
            hotham_queries: Queries::default(),
        })
    }

    fn begin_frame(&mut self) {
        begin_frame(
            &mut self.engine.xr_context,
            &self.engine.vulkan_context,
            &self.engine.render_context,
        )
    }

    fn run(&mut self) -> HothamResult<()> {
        while let Ok((_, _)) = self.engine.update() {
            self.begin_frame();
            let engine = &mut self.engine;
            let world = &mut self.world;
            let queries = &mut self.hotham_queries;
            let xr_context = &mut engine.xr_context;
            let vulkan_context = &engine.vulkan_context;
            let render_context = &mut engine.render_context;
            let physics_context = &mut engine.physics_context;
            let gui_context = &mut engine.gui_context;
            let haptic_context = &mut engine.haptic_context;

            hands_system(&mut queries.hands_query, world, xr_context, physics_context);
            physics_step(physics_context);
            collision_system(&mut queries.collision_query, world, physics_context);
            pointers_system(
                &mut queries.pointers_query,
                world,
                xr_context,
                physics_context,
            );
            update_rigid_body_transforms_system(
                &mut queries.update_rigid_body_transforms_query,
                world,
                physics_context,
            );
            animation_system(&mut queries.animation_query, world);
            draw_gui_system(
                &mut queries.draw_gui_query,
                world,
                vulkan_context,
                &xr_context.frame_index,
                render_context,
                gui_context,
                haptic_context,
            );
            update_transform_matrix_system(&mut queries.update_transform_matrix_query, world);
            update_parent_transform_matrix_system(
                &mut queries.parent_query,
                &mut queries.roots_query,
                world,
            );
            skinning_system(&mut queries.joints_query, &mut queries.meshes_query, world);
            begin_pbr_renderpass(xr_context, vulkan_context, render_context);
            rendering_system(
                &mut queries.rendering_query,
                world,
                vulkan_context,
                xr_context.frame_index,
                render_context,
            );
            end_pbr_renderpass(xr_context, vulkan_context, render_context);
            end_frame(xr_context, vulkan_context, render_context);
        }

        Ok(())
    }
}
